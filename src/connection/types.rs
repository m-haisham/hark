use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(String);

impl From<String> for ConnectionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(flatten)]
    pub auth: ConnectionAuth,
    pub mailbox: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum ConnectionAuth {
    Password { password: String },
    Xoauth2 { token: String },
}

#[derive(Debug)]
pub enum ConnectionCommandIn {
    Stop,
}

#[derive(Debug)]
pub enum ConnectionCommandOut {}
