mod handle;

use chrono::{DateTime, Utc};
use oauth2::{AccessToken, AuthUrl, ClientId, ClientSecret, RefreshToken, TokenUrl};
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub use handle::ConnectionHandle;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(String);

impl Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "connection:{}", self.0)
    }
}

impl TryFrom<String> for ConnectionId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("Connection id cannot be empty");
        }

        // should not be longer than 20 characters
        if value.len() > 20 {
            return Err("Connection id cannot be longer than 20 characters");
        }

        // should start with an alphabet
        if let Some(char) = value.chars().next() {
            if !char.is_alphabetic() {
                return Err("Connection id should start with an alphabet");
            }
        }

        // should only contain alphanumeric characters and underscores
        if !value.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Connection id can only contain alphanumeric characters and underscores");
        }

        Ok(Self(value))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(flatten)]
    pub auth: ConnectionAuth,
    pub tls: bool,
    pub mailbox: String,
    #[serde(default)]
    pub flavour: Option<ImapFlavour>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum ConnectionAuth {
    Password {
        #[serde(skip_serializing)]
        password: Secret<String>,
    },
    OAuth2(OAuth2),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuth2 {
    #[serde(skip_serializing)]
    pub access_token: AccessToken,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub refresh_token: RefreshToken,
    pub config: OAuth2Config,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuth2Config {
    #[serde(skip_serializing)]
    pub client_id: ClientId,
    #[serde(skip_serializing)]
    pub client_secret: ClientSecret,
    pub auth_uri: AuthUrl,
    pub token_uri: TokenUrl,
    pub scope: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImapFlavour {
    Gmail,
}

impl Display for ImapFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImapFlavour::Gmail => write!(f, "google"),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum ConnectionState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug)]
pub enum ConnectionCommand {
    Stop,
}

#[derive(Debug)]
pub struct ConnectionEvent {
    pub id: ConnectionId,
    pub event: ConnectionEventKind,
}

#[derive(Debug)]
pub enum ConnectionEventKind {
    Started,
    Stopped,
}
