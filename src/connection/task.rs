use std::{sync::Arc, time};

use anyhow::Context;
use chrono::Duration;
use futures::lock::Mutex;
use stop_token::StopSource;
use tokio::sync::mpsc;

use crate::imap::{
    handle_idle_event, handle_idle_response, imap_connect, imap_idle, imap_listen, ImapAuth,
    ImapConnectionConfig, ImapListenConfig,
};

use super::types::{Connection, ConnectionAuth, ConnectionCommandIn, ConnectionId};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: ConnectionId,
    pub connection: Connection,
    pub receiver: mpsc::Receiver<ConnectionCommandIn>,
}

#[derive(Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    pub sender: mpsc::Sender<ConnectionCommandIn>,
}

#[derive(Debug)]
pub struct ConnectionTaskState {
    pub stop: bool,
}

async fn listen_receiver(
    mut receiver: mpsc::Receiver<ConnectionCommandIn>,
    id: ConnectionId,
    state: Arc<Mutex<ConnectionTaskState>>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ConnectionCommandIn::Stop => {
                let mut state = state.lock().await;
                state.stop = true;
                return;
            }
        }
    }
}

#[tracing::instrument]
pub async fn run_connection_task(task: ConnectionTask) -> anyhow::Result<()> {
    let ConnectionTask {
        id,
        connection,
        receiver,
    } = task;

    let state = Arc::new(Mutex::new(ConnectionTaskState { stop: false }));
    tokio::spawn(listen_receiver(receiver, id.clone(), state.clone()));

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
        let mut idle = session.idle();
        idle.init().await?;

        let (idle_wait, interrupt) = idle.wait();

        // check state for stop every 10 seconds and drop interrupt if stop is true
        // while simultaneously waiting for idle_wait to complete
        let response = tokio::select! {
            response = idle_wait => response,
            _ = drop_interrupt_when_stopped(state.clone(), interrupt) => break,
        }?;

        let (idle, result) = handle_idle_response(idle, response).await?;

        session = idle.done().await?;

        let messages = handle_idle_event(&mut session, &mut listen, result).await?;
        for message in messages {
            println!("Received message: {:#?}", message);
        }
    }

    Ok(())
}

async fn drop_interrupt_when_stopped(
    state: Arc<Mutex<ConnectionTaskState>>,
    interrupt: StopSource,
) {
    loop {
        let stop = {
            let state = state.lock().await;
            state.stop
        };

        if stop {
            drop(interrupt);
            break;
        }

        tokio::time::sleep(time::Duration::from_secs(10)).await;
    }
}
