mod body;

use std::{borrow::Cow, string::FromUtf8Error};

use async_imap::{
    extensions::idle::IdleResponse, types::UnsolicitedResponse, Client, Connection, Session,
};
use async_native_tls::TlsStream;
use chrono::{Duration, Utc};
use itertools::Itertools;
use tokio::net::TcpStream;

use crate::{imap::body::parse_body, types::Message};

#[cfg(not(debug_assertions))]
pub type ImapStream = TlsStream<TcpStream>;

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
    session: Session<ImapStream>,
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
) -> Result<ImapListen, ImapError> {
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

    Ok(ImapListen {
        session,
        config,
        size: mailbox.exists,
        state,
    })
}

// impl Iterator for ImapListen {
//     type Item = Result<Vec<Message>, ImapListenError>;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.state {
//             ImapListenState::Lookback(duration) => {
//                 let result = imap_lookback(&mut self.session, duration);

//                 match &result {
//                     Ok(_) => self.state = ImapListenState::Idle,
//                     Err(_) => self.state = ImapListenState::Error,
//                 };

//                 Some(result)
//             }
//             ImapListenState::Idle => {
//                 let result = imap_idle(self);

//                 match &result {
//                     Ok(_) => self.state = ImapListenState::Idle,
//                     Err(_) => self.state = ImapListenState::Error,
//                 };

//                 Some(result)
//             }
//             ImapListenState::Error => {
//                 self.state = ImapListenState::Error;
//                 None
//             }
//         }
//     }
// }

// async fn imap_lookback(
//     session: &mut Session<ImapStream>,
//     duration: Duration,
// ) -> Result<Vec<Message>, ImapListenError> {
//     let from_date = Utc::now() - duration;
//     let formatted = from_date.format("%d-%b-%Y");
//     let uids = session.search(format!("UNSEEN SINCE {formatted}")).await?;

//     let seq: String = uids.into_iter().map(|v| v.to_string()).join(",");
//     fetch_seq(session, &seq)
// }

pub async fn imap_idle(session: Session<ImapStream>) -> Result<Vec<Message>, ImapListenError> {
    let mut idle = session.idle();
    idle.init().await?;

    let (idle_wait, interrupt) = idle.wait();

    let idle_result = idle_wait.await?;
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
            let s = String::from_utf8(data.borrow_owner().to_vec()).unwrap();
            return Err(ImapListenError::Exit);
        }
    }
}

enum IdleEvent {
    Exit,
    Exists(u32),
    Fetch(u32),
    SizeDecrease,
}

// fn fetch_seq(
//     session: &mut Session<ImapStream>,
//     seq: &str,
// ) -> Result<Vec<Message>, ImapListenError> {
//     let messages = session.fetch(seq, "(FLAGS INTERNALDATE BODY[] UID)")?;

//     let mut parsed = vec![];
//     for message in messages.iter() {
//         let Some(body) = message.body() else {
//             continue;
//         };

//         let parsed_message = parse_body(body).unwrap();
//         parsed.push(parsed_message);
//     }

//     Ok(parsed)
// }
