use serde::Deserialize;

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
pub enum ConnectionCommand {}
