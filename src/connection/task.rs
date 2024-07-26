use anyhow::Context;
use chrono::Duration;
use tokio::sync::mpsc;

use crate::{
    imap::{
        imap_connect, imap_idle, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig,
    },
    task::TaskId,
};

use super::types::{Connection, ConnectionAuth, ConnectionCommand};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: TaskId,
    pub key: String,
    pub connection: Connection,
    pub receiver: mpsc::Receiver<ConnectionCommand>,
}

#[derive(Debug)]
pub struct ConnectionHandle {
    pub key: String,
    pub sender: mpsc::Sender<ConnectionCommand>,
}

#[tracing::instrument]
pub async fn run_connection_task(task: ConnectionTask) -> anyhow::Result<()> {
    let connection = task.connection;

    let auth = match connection.auth {
        ConnectionAuth::Password { password } => ImapAuth::LOGIN {
            username: connection.username,
            password,
        },
        ConnectionAuth::Xoauth2 { token } => ImapAuth::XOAUTH2 {
            username: connection.username,
            access_token: token,
        },
    };

    let imap_connection = ImapConnectionConfig {
        host: connection.host,
        port: connection.port,
        auth,
    };

    let session = imap_connect(&imap_connection)
        .await
        .context("Failed to connect to IMAP server")?;

    let listen_config = ImapListenConfig {
        mailbox: connection.mailbox,
        lookback_duration: Some(Duration::try_days(30).unwrap()),
    };

    let (mut session, mut listen) = imap_listen(session, listen_config)
        .await
        .context("Failed to start listening to IMAP server")?;

    loop {
        let (returned_session, messages) = imap_idle(&mut listen, session)
            .await
            .context("Failed to idle")?;

        session = returned_session;

        for message in messages {
            println!("Received message: {:#?}", message);
        }
    }
}
