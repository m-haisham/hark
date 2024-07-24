mod body;

use futures::StreamExt;
use std::{borrow::Cow, string::FromUtf8Error};

use async_imap::{extensions::idle::IdleResponse, Client, Session};
use chrono::{Duration, Utc};
use imap_proto::{MailboxDatum, Response};
use itertools::Itertools;
use tokio::net::TcpStream;

use crate::{imap::body::parse_body, types::Message};

#[cfg(not(debug_assertions))]
pub type ImapStream = async_native_tls::TlsStream<TcpStream>;

#[cfg(debug_assertions)]
pub type ImapStream = TcpStream;

pub struct ImapConnectionConfig {
    pub host: String,
    pub port: u16,
    pub auth: ImapAuth,
}

pub enum ImapAuth {
    LOGIN {
        username: String,
        password: String,
    },
    XOAUTH2 {
        username: String,
        access_token: String,
    },
}

pub struct ImapListenConfig {
    pub mailbox: String,
    pub lookback_duration: Option<Duration>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImapError {
    #[error("{0}")]
    Imap(#[from] async_imap::error::Error),

    #[error("Imap server does not define the capability: {0}")]
    LackingCapability(String),
}

struct XOAuth2Authenticator<'a> {
    user: &'a str,
    access_token: &'a str,
}

impl async_imap::Authenticator for XOAuth2Authenticator<'_> {
    type Response = String;
    fn process(&mut self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

pub async fn imap_connect(config: &ImapConnectionConfig) -> Result<Session<ImapStream>, ImapError> {
    let stream = TcpStream::connect((config.host.as_str(), config.port))
        .await
        .unwrap();

    #[cfg(not(debug_assertions))]
    let stream = async_native_tls::connect(&config.host, stream)
        .await
        .unwrap();

    let client = async_imap::Client::new(stream);
    imap_auth(client, &config.auth).await
}

pub async fn imap_auth(
    mut client: Client<ImapStream>,
    auth: &ImapAuth,
) -> Result<Session<ImapStream>, ImapError> {
    match &auth {
        ImapAuth::LOGIN { username, password } => {
            check_auth_capability(&mut client, "LOGIN")?;
            client
                .login(username, password)
                .await
                .map_err(|(e, _)| e.into())
        }
        ImapAuth::XOAUTH2 {
            username,
            access_token,
        } => {
            check_auth_capability(&mut client, "XOAUTH2")?;

            let cred = XOAuth2Authenticator {
                user: username,
                access_token,
            };

            client
                .authenticate("XOAUTH2", cred)
                .await
                .map_err(|(e, _)| e.into())
        }
    }
}

fn check_auth_capability(
    client: &mut Client<ImapStream>,
    capability_str: &str,
) -> Result<(), ImapError> {
    let capability = &imap_proto::Capability::Auth(Cow::Borrowed(capability_str));

    // if !client.capabilities()?.has(capability) {
    //     return Err(ImapError::LackingCapability(capability_str.to_string()));
    // }

    Ok(())
}

pub struct ImapListen {
    config: ImapListenConfig,
    size: u32,
    state: ImapListenState,
}

pub enum ImapListenState {
    Lookback(Duration),
    Idle,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum ImapListenError {
    #[error("{0}")]
    Imap(#[from] async_imap::error::Error),

    #[error("{0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("{0}")]
    DateError(#[from] chrono::ParseError),

    #[error("Imap server idle send 'EXIT'")]
    Exit,
}

pub async fn imap_listen(
    mut session: Session<ImapStream>,
    config: ImapListenConfig,
) -> Result<(Session<ImapStream>, ImapListen), ImapError> {
    let capability = async_imap::types::Capability::Atom(String::from("IDLE"));

    if !session.capabilities().await?.has(&capability) {
        return Err(ImapError::LackingCapability("IDLE".to_string()));
    }

    let mailbox = session.select(&config.mailbox).await?;

    let state = match &config.lookback_duration {
        Some(duration) => ImapListenState::Lookback(duration.clone()),
        None => ImapListenState::Idle,
    };

    println!("Listening to mailbox: {}", config.mailbox);

    Ok((
        session,
        ImapListen {
            config,
            size: mailbox.exists,
            state,
        },
    ))
}

async fn imap_lookback(
    session: &mut Session<ImapStream>,
    duration: Duration,
) -> Result<Vec<Message>, ImapListenError> {
    let from_date = Utc::now() - duration;
    let formatted = from_date.format("%d-%b-%Y");
    let uids = session.search(format!("UNSEEN SINCE {formatted}")).await?;

    let seq: String = uids.into_iter().map(|v| v.to_string()).join(",");
    fetch_seq(session, &seq).await
}

pub async fn imap_idle(
    listen: &mut ImapListen,
    mut session: Session<ImapStream>,
) -> Result<(Session<ImapStream>, Vec<Message>), ImapListenError> {
    let mut idle = session.idle();
    idle.init().await?;

    let (idle_wait, interrupt) = idle.wait();

    let idle_result = idle_wait.await?;

    let mut result = IdleEvent::Exit;
    match idle_result {
        IdleResponse::ManualInterrupt => {
            idle.done().await?;
            return Err(ImapListenError::Exit);
        }
        IdleResponse::Timeout => {
            idle.done().await?;
            return Err(ImapListenError::Exit);
        }
        IdleResponse::NewData(data) => {
            println!("New data: {:?}", data.parsed());
            if let Some(event) = parse_response(data.parsed()) {
                result = event;
            }
        }
    }

    session = idle.done().await?;

    match result {
        IdleEvent::Exit => {
            return Err(ImapListenError::Exit);
        }
        IdleEvent::SizeDecrease => {
            let mailbox = session.select(&listen.config.mailbox).await?;
            listen.size = mailbox.exists;
            return Ok((session, vec![]));
        }
        IdleEvent::Exists(new_size) => {
            let seq = format!("{}:{}", listen.size + 1, new_size);
            listen.size = new_size;
            let messages = fetch_seq(&mut session, &seq).await?;
            return Ok((session, messages));
        }
        IdleEvent::Fetch(id) => {
            let mailbox = session.select(&listen.config.mailbox).await?;
            listen.size = mailbox.exists;
            let messages = fetch_seq(&mut session, &id.to_string()).await?;
            return Ok((session, messages));
        }
    };
}

fn parse_response(response: &Response) -> Option<IdleEvent> {
    match response {
        Response::Capabilities(_) => None,
        Response::Continue { .. } => None,
        Response::Done { .. } => None,
        Response::Data { .. } => None,
        Response::Expunge(_) => Some(IdleEvent::SizeDecrease),
        Response::Vanished { .. } => Some(IdleEvent::SizeDecrease),
        Response::Fetch(uid, _) => Some(IdleEvent::Fetch(*uid)),
        Response::MailboxData(mailbox) => match mailbox {
            MailboxDatum::Exists(uid) => Some(IdleEvent::Exists(*uid)),
            MailboxDatum::Flags(_) => None,
            MailboxDatum::List { .. } => None,
            MailboxDatum::Search(_) => None,
            MailboxDatum::Sort(_) => None,
            MailboxDatum::Status { .. } => None,
            MailboxDatum::Recent(_) => None,
            MailboxDatum::MetadataSolicited { .. } => None,
            MailboxDatum::MetadataUnsolicited { .. } => None,
            MailboxDatum::GmailLabels(_) => None,
            MailboxDatum::GmailMsgId(_) => None,
            _ => None,
        },
        Response::Quota(_) => None,
        Response::QuotaRoot(_) => None,
        Response::Id(_) => None,
        Response::Acl(_) => None,
        Response::ListRights(_) => None,
        Response::MyRights(_) => None,
        _ => None,
    }
}

enum IdleEvent {
    Exit,
    Exists(u32),
    Fetch(u32),
    SizeDecrease,
}

async fn fetch_seq(
    session: &mut Session<ImapStream>,
    seq: &str,
) -> Result<Vec<Message>, ImapListenError> {
    let mut messages = session
        .fetch(seq, "(FLAGS INTERNALDATE RFC822 BODY[] UID)")
        .await?;

    let mut parsed = vec![];
    while let Some(message) = messages.next().await {
        let message = message?;

        let Some(body) = message.body() else {
            continue;
        };

        let parsed_message = parse_body(body).unwrap();
        parsed.push(parsed_message);
    }

    Ok(parsed)
}
