mod body;

use std::{borrow::Cow, string::FromUtf8Error};

use chrono::{Duration, Utc};
use imap::{types::UnsolicitedResponse, Client, ImapConnection, Session};
use itertools::Itertools;

use crate::{imap::body::parse_body, types::Message};

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
    Imap(#[from] imap::Error),

    #[error("Imap server does not define the capability: {0}")]
    LackingCapability(String),
}

struct XOAuth2Authenticator<'a> {
    user: &'a str,
    access_token: &'a str,
}

impl imap::Authenticator for XOAuth2Authenticator<'_> {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

pub fn imap_connect(
    config: &ImapConnectionConfig,
) -> Result<Session<Box<dyn ImapConnection>>, ImapError> {
    let mut client = imap::ClientBuilder::new(&config.host, config.port).connect()?;

    #[cfg(debug_assertions)]
    {
        client.debug = true;
    }

    imap_auth(client, &config.auth)
}

pub fn imap_auth(
    mut client: Client<Box<dyn ImapConnection>>,
    auth: &ImapAuth,
) -> Result<Session<Box<dyn ImapConnection>>, ImapError> {
    match &auth {
        ImapAuth::LOGIN { username, password } => {
            check_auth_capability(&mut client, "LOGIN")?;
            client.login(username, password).map_err(|(e, _)| e.into())
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
                .authenticate("XOAUTH2", &cred)
                .map_err(|(e, _)| e.into())
        }
    }
}

fn check_auth_capability(
    client: &mut Client<Box<dyn ImapConnection>>,
    capability_str: &str,
) -> Result<(), ImapError> {
    let capability = &imap_proto::Capability::Auth(Cow::Borrowed(capability_str));

    if !client.capabilities()?.has(capability) {
        return Err(ImapError::LackingCapability("XOAUTH2".to_string()));
    }

    Ok(())
}

pub struct ImapListen {
    session: Session<Box<dyn ImapConnection>>,
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
    Imap(#[from] imap::Error),

    #[error("{0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("{0}")]
    DateError(#[from] chrono::ParseError),

    #[error("Imap server idle send 'EXIT'")]
    Exit,
}

pub fn imap_listen(
    mut session: Session<Box<dyn ImapConnection>>,
    config: ImapListenConfig,
) -> Result<ImapListen, ImapError> {
    let capability = imap_proto::Capability::Atom(Cow::Borrowed("IDLE"));
    if !session.capabilities()?.has(&capability) {
        return Err(ImapError::LackingCapability("IDLE".to_string()));
    }

    let mailbox = session.select(&config.mailbox)?;

    let state = match &config.lookback_duration {
        Some(duration) => ImapListenState::Lookback(duration.clone()),
        None => ImapListenState::Idle,
    };

    Ok(ImapListen {
        session,
        config,
        size: mailbox.exists,
        state,
    })
}

impl Iterator for ImapListen {
    type Item = Result<Vec<Message>, ImapListenError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            ImapListenState::Lookback(duration) => {
                let result = imap_lookback(&mut self.session, duration);

                match &result {
                    Ok(_) => self.state = ImapListenState::Idle,
                    Err(_) => self.state = ImapListenState::Error,
                };

                Some(result)
            }
            ImapListenState::Idle => {
                let result = imap_idle(self);

                match &result {
                    Ok(_) => self.state = ImapListenState::Idle,
                    Err(_) => self.state = ImapListenState::Error,
                };

                Some(result)
            }
            ImapListenState::Error => {
                self.state = ImapListenState::Error;
                None
            }
        }
    }
}

fn imap_lookback(
    session: &mut Session<Box<dyn ImapConnection>>,
    duration: Duration,
) -> Result<Vec<Message>, ImapListenError> {
    let from_date = Utc::now() - duration;
    let formatted = from_date.format("%d-%b-%Y");
    let uids = session.search(format!("UNSEEN SINCE {formatted}"))?;

    let seq: String = uids.into_iter().map(|v| v.to_string()).join(",");
    fetch_seq(session, &seq)
}

fn imap_idle(listen: &mut ImapListen) -> Result<Vec<Message>, ImapListenError> {
    // idle and watch for new emails
    loop {
        let mut result = IdleEvent::Exit;

        listen.session.idle().wait_while(|r| match r {
            UnsolicitedResponse::Bye { .. } => false,
            UnsolicitedResponse::Exists(size) => {
                result = IdleEvent::Exists(size);
                false
            }
            UnsolicitedResponse::Expunge(_) => {
                result = IdleEvent::SizeDecrease;
                false
            }
            UnsolicitedResponse::Fetch { id, .. } => {
                result = IdleEvent::Fetch(id);
                false
            }
            UnsolicitedResponse::Flags(_) => true,
            UnsolicitedResponse::Metadata { .. } => true,
            UnsolicitedResponse::Ok { .. } => true,
            UnsolicitedResponse::Recent(_) => true,
            UnsolicitedResponse::Status { .. } => true,
            UnsolicitedResponse::Vanished { .. } => {
                result = IdleEvent::SizeDecrease;
                false
            }
            _ => true,
        })?;

        match result {
            IdleEvent::Exit => {
                listen.session.logout()?;
                return Err(ImapListenError::Exit);
            }
            IdleEvent::SizeDecrease => {
                let mailbox = listen.session.select(&listen.config.mailbox)?;
                listen.size = mailbox.exists;
            }
            IdleEvent::Exists(new_size) => {
                let seq = format!("{}:{}", listen.size + 1, new_size);
                listen.size = new_size;
                return fetch_seq(&mut listen.session, &seq);
            }
            IdleEvent::Fetch(id) => {
                let mailbox = listen.session.select(&listen.config.mailbox)?;
                listen.size = mailbox.exists;
                return fetch_seq(&mut listen.session, &id.to_string());
            }
        };
    }
}

enum IdleEvent {
    Exit,
    Exists(u32),
    Fetch(u32),
    SizeDecrease,
}

fn fetch_seq(
    session: &mut Session<Box<dyn ImapConnection>>,
    seq: &str,
) -> Result<Vec<Message>, ImapListenError> {
    let messages = session.fetch(seq, "(FLAGS INTERNALDATE BODY[] UID)")?;

    let mut parsed = vec![];
    for message in messages.iter() {
        let Some(body) = message.body() else {
            continue;
        };

        let parsed_message = parse_body(body).unwrap();
        parsed.push(parsed_message);
    }

    Ok(parsed)
}
