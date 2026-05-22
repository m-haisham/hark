use async_imap::{
    types::{Capabilities, Capability, Mailbox, UnsolicitedResponse},
    Session,
};
use eyre::{eyre, Context};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use crate::imap::{
    connect::{imap_connect_tcp, imap_connect_tls, ImapConnectionConfig},
    fetch_seq, MessageParseResult,
};

pub type TcpConnection = TcpStream;
pub type TlsConnection = TlsStream<TcpStream>;

#[derive(Debug)]
pub enum ImapSession {
    Tcp(Session<TcpConnection>),
    Tls(Session<TlsConnection>),
}

impl ImapSession {
    pub async fn connect(config: &ImapConnectionConfig) -> eyre::Result<Self> {
        if config.tls {
            tracing::info!("Connecting to IMAP server with TLS");
            let session = imap_connect_tls(&config).await?;
            Ok(ImapSession::Tls(session))
        } else {
            tracing::info!("Connecting to IMAP server with TCP");
            let session = imap_connect_tcp(&config).await?;
            Ok(ImapSession::Tcp(session))
        }
    }

    pub async fn select(&mut self, mailbox: &str) -> eyre::Result<Mailbox> {
        match self {
            ImapSession::Tcp(session) => session.select(mailbox).await,
            ImapSession::Tls(session) => session.select(mailbox).await,
        }
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to select mailbox")
    }

    pub async fn noop(&mut self) -> async_imap::error::Result<()> {
        match self {
            ImapSession::Tcp(session) => session.noop().await,
            ImapSession::Tls(session) => session.noop().await,
        }
    }

    pub async fn capabilities(&mut self) -> eyre::Result<Capabilities> {
        match self {
            ImapSession::Tcp(session) => session.capabilities().await,
            ImapSession::Tls(session) => session.capabilities().await,
        }
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to get IMAP capabilities")
    }

    pub async fn has_idle_capability(&mut self) -> eyre::Result<bool> {
        let caps = self.capabilities().await?;
        Ok(caps.has(&Capability::Atom("IDLE".to_string())))
    }

    pub async fn unsolicited_responses(&mut self) -> &async_channel::Receiver<UnsolicitedResponse> {
        match self {
            ImapSession::Tcp(session) => &session.unsolicited_responses,
            ImapSession::Tls(session) => &session.unsolicited_responses,
        }
    }

    pub async fn fetch_messages(
        &mut self,
        sequence_set: &str,
    ) -> async_imap::error::Result<Vec<MessageParseResult>> {
        match self {
            ImapSession::Tcp(session) => fetch_seq(session, sequence_set).await,
            ImapSession::Tls(session) => fetch_seq(session, sequence_set).await,
        }
    }

    pub async fn logout(&mut self) -> async_imap::error::Result<()> {
        match self {
            ImapSession::Tcp(session) => session.logout().await,
            ImapSession::Tls(session) => session.logout().await,
        }
    }
}
