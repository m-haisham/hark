use async_imap::Session;
use async_native_tls::TlsStream;
use tokio::net::TcpStream;

use crate::imap::{
    connect::{imap_connect_tcp, imap_connect_tls, ImapConnectionConfig},
    fetch_seq, MessageParseResult,
};

pub type TcpConnection = TcpStream;
pub type TlsConnection = TlsStream<TcpStream>;

pub enum ImapSession {
    Tcp(Session<TcpConnection>),
    Tls(Session<TlsConnection>),
}

impl ImapSession {
    pub async fn connect(config: ImapConnectionConfig) -> eyre::Result<Self> {
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
