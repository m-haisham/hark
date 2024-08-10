mod body;
pub mod types;

use anyhow::Context;
use async_native_tls::TlsStream;
use futures::StreamExt;
use oauth2::AccessToken;
use secrecy::{ExposeSecret, Secret};
use std::{borrow::Cow, fmt::Debug, string::FromUtf8Error};

use async_imap::{
    extensions::idle::{self, IdleResponse},
    Client, Session,
};
use chrono::{Duration, Utc};
use imap_proto::{MailboxDatum, Response};
use itertools::Itertools;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::{connection::types::ImapFlavour, imap::body::parse_body};
use types::Message;

#[derive(Debug)]
pub struct ImapConnectionConfig {
    pub host: String,
    pub port: u16,
    pub auth: ImapAuth,
    pub flavour: Option<ImapFlavour>,
}

#[derive(Debug)]
pub enum ImapAuth {
    LOGIN {
        username: String,
        password: Secret<String>,
    },
    XOAUTH2 {
        username: String,
        access_token: AccessToken,
    },
}

#[derive(Debug)]
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

    #[error("{0}")]
    AuthFailed(#[source] async_imap::error::Error),
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

#[tracing::instrument(
    name = "IMAP Connect with TLS",
    skip(config),
    fields(host = %config.host.as_str(), port = %config.port)
)]
pub async fn imap_connect_tls(
    config: &ImapConnectionConfig,
) -> anyhow::Result<Session<TlsStream<TcpStream>>> {
    let addr = (config.host.as_str(), config.port);
    let stream = TcpStream::connect(addr).await.with_context(|| {
        format!(
            "Failed to connect to IMAP server at {}:{}",
            config.host, config.port
        )
    })?;

    let stream = async_native_tls::connect(&config.host, stream)
        .await
        .with_context(|| {
            format!(
                "Failed to establish TLS connection to IMAP server at {}:{}",
                config.host, config.port
            )
        })?;

    let client = create_client(config, stream).await?;

    imap_auth(client, &config.auth)
        .await
        .context("Failed to authenticate with IMAP server")
}

#[tracing::instrument(
    name = "IMAP Connect with TCP",
    skip(config),
    fields(host = %config.host.as_str(), port = %config.port)
)]
pub async fn imap_connect_tcp(config: &ImapConnectionConfig) -> anyhow::Result<Session<TcpStream>> {
    let addr = (config.host.as_str(), config.port);
    let stream = TcpStream::connect(addr).await.with_context(|| {
        format!(
            "Failed to connect to IMAP server at {}:{}",
            config.host, config.port
        )
    })?;

    let client = create_client(config, stream).await?;

    imap_auth(client, &config.auth)
        .await
        .context("Failed to authenticate with IMAP server")
}

#[tracing::instrument(
    name = "IMAP Create Client",
    skip_all,
    fields(
        host = %config.host.as_str(),
        port = %config.port,
        flavour = ?config.flavour
    )
)]
async fn create_client<T>(config: &ImapConnectionConfig, stream: T) -> anyhow::Result<Client<T>>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let mut client = async_imap::Client::new(stream);

    if let Some(ImapFlavour::Gmail) = config.flavour {
        tracing::info!("Gmail IMAP flavour detected, receiving greeting");

        client
            .read_response()
            .await
            .context("Expected greeting response from gmail IMAP server")?
            .context("Failed to read response")?;
    }

    Ok(client)
}

#[tracing::instrument(name = "IMAP Authenticate", skip(client))]
pub async fn imap_auth<T>(mut client: Client<T>, auth: &ImapAuth) -> Result<Session<T>, ImapError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match &auth {
        ImapAuth::LOGIN { username, password } => {
            check_auth_capability(&mut client, "LOGIN")?;
            client
                .login(username, password.expose_secret())
                .await
                .map_err(parse_auth_error)
        }
        ImapAuth::XOAUTH2 {
            username,
            access_token,
        } => {
            check_auth_capability(&mut client, "XOAUTH2")?;

            let cred = XOAuth2Authenticator {
                user: username,
                access_token: access_token.secret(),
            };

            client
                .authenticate("XOAUTH2", cred)
                .await
                .map_err(parse_auth_error)
        }
    }
}

pub fn parse_auth_error<T>((error, _client): (async_imap::error::Error, Client<T>)) -> ImapError
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match error {
        async_imap::error::Error::No(ref e) if e.contains("AUTHENTICATIONFAILED") => {
            ImapError::AuthFailed(error)
        }
        _ => ImapError::Imap(error),
    }
}

#[tracing::instrument(skip(client))]
fn check_auth_capability<T>(client: &mut Client<T>, capability_str: &str) -> Result<(), ImapError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let capability = &imap_proto::Capability::Auth(Cow::Borrowed(capability_str));

    // if !client.capabilities()?.has(capability) {
    //     return Err(ImapError::LackingCapability(capability_str.to_string()));
    // }

    Ok(())
}

#[derive(Debug)]
pub struct ImapListen {
    config: ImapListenConfig,
    size: u32,
    state: ImapListenState,
}

#[derive(Debug)]
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

#[tracing::instrument(skip(session))]
pub async fn imap_listen<T>(
    mut session: Session<T>,
    config: ImapListenConfig,
) -> Result<(Session<T>, ImapListen), ImapError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let capability = async_imap::types::Capability::Atom(String::from("IDLE"));

    if !session.capabilities().await?.has(&capability) {
        return Err(ImapError::LackingCapability("IDLE".to_string()));
    }

    let mailbox = session.select(&config.mailbox).await?;

    let state = match &config.lookback_duration {
        Some(duration) => ImapListenState::Lookback(duration.clone()),
        None => ImapListenState::Idle,
    };

    Ok((
        session,
        ImapListen {
            config,
            size: mailbox.exists,
            state,
        },
    ))
}

async fn imap_lookback<T>(
    session: &mut Session<T>,
    duration: Duration,
) -> Result<Vec<Message>, ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let from_date = Utc::now() - duration;
    let formatted = from_date.format("%d-%b-%Y");
    let uids = session.search(format!("UNSEEN SINCE {formatted}")).await?;

    let seq: String = uids.into_iter().map(|v| v.to_string()).join(",");
    fetch_seq(session, &seq).await
}

#[tracing::instrument(skip(session))]
pub async fn imap_idle<T>(
    listen: &mut ImapListen,
    mut session: Session<T>,
) -> Result<(Session<T>, Vec<Message>), ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let mut idle = session.idle();
    idle.init().await?;

    let (idle_wait, _) = idle.wait();

    let response = idle_wait.await?;
    let (idle, result) = handle_idle_response(idle, response).await?;

    session = idle.done().await?;
    let messages = handle_idle_event(&mut session, listen, result).await?;

    Ok((session, messages))
}

pub async fn handle_idle_response<T>(
    idle: idle::Handle<T>,
    response: IdleResponse,
) -> Result<(idle::Handle<T>, IdleEvent), ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match response {
        IdleResponse::ManualInterrupt => {
            idle.done().await?;
            Err(ImapListenError::Exit)
        }
        IdleResponse::Timeout => {
            idle.done().await?;
            Err(ImapListenError::Exit)
        }
        IdleResponse::NewData(data) => {
            tracing::debug!("New data: {:?}", data);
            if let Some(event) = parse_response(data.parsed()) {
                Ok((idle, event))
            } else {
                Err(ImapListenError::Exit)
            }
        }
    }
}

pub async fn handle_idle_event<T>(
    session: &mut Session<T>,
    listen: &mut ImapListen,
    event: IdleEvent,
) -> Result<Vec<Message>, ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match event {
        IdleEvent::Exit => {
            return Err(ImapListenError::Exit);
        }
        IdleEvent::SizeDecrease => {
            let mailbox = session.select(&listen.config.mailbox).await?;
            listen.size = mailbox.exists;
            return Ok(vec![]);
        }
        IdleEvent::Exists(new_size) => {
            let seq = format!("{}:{}", listen.size + 1, new_size);
            listen.size = new_size;
            let messages = fetch_seq(session, &seq).await?;
            return Ok(messages);
        }
        IdleEvent::Fetch(id) => {
            let mailbox = session.select(&listen.config.mailbox).await?;
            listen.size = mailbox.exists;
            let messages = fetch_seq(session, &id.to_string()).await?;
            return Ok(messages);
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

pub enum IdleEvent {
    Exit,
    Exists(u32),
    Fetch(u32),
    SizeDecrease,
}

#[tracing::instrument(skip(session))]
async fn fetch_seq<T>(session: &mut Session<T>, seq: &str) -> Result<Vec<Message>, ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
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
