use std::fmt::Display;

use serde::{Deserialize, Serialize};

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
    pub mailbox: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum ConnectionAuth {
    Password { password: String },
    Xoauth2 { token: String },
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
    id: ConnectionId,
    event: ConnectionEventKind,
}

#[derive(Debug)]
pub enum ConnectionEventKind {
    Stopped,
}
