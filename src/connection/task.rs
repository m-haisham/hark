use std::{sync::Arc, time};

use anyhow::Context;
use chrono::Duration;
use futures::lock::Mutex;
use secrecy::ExposeSecret;
use stop_token::StopSource;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::imap::{
    handle_idle_event, handle_idle_response, imap_connect, imap_listen, ImapAuth,
    ImapConnectionConfig, ImapListenConfig,
};

use super::types::{Connection, ConnectionAuth, ConnectionCommand, ConnectionId};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: ConnectionId,
    pub connection: Connection,
    pub receiver: mpsc::Receiver<ConnectionCommand>,
}

#[derive(Debug)]
pub struct ConnectionTaskState {
    pub stop: bool,
}

#[instrument(name = "Listen for commands to connection", skip_all, fields(id = %id))]
async fn listen_command(
    mut receiver: mpsc::Receiver<ConnectionCommand>,
    id: ConnectionId,
    state: Arc<Mutex<ConnectionTaskState>>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ConnectionCommand::Stop => {
                let mut state = state.lock().await;
                state.stop = true;
                tracing::info!("Connection task marked for stop");
                return;
            }
        }
    }
}

#[tracing::instrument(name = "Connection Task", skip(task), fields(id = %task.id))]
pub async fn run_connection_task(task: ConnectionTask) {
    let result = run_connection_task_inner(task).await;
    if let Err(err) = result {
        tracing::error!("Connection task failed: {:?}", err);
    }
}

pub async fn run_connection_task_inner(task: ConnectionTask) -> anyhow::Result<()> {
    let ConnectionTask {
        id,
        connection,
        receiver,
    } = task;

    let state = Arc::new(Mutex::new(ConnectionTaskState { stop: false }));
    let listen_handle = tokio::spawn(listen_command(receiver, id.clone(), state.clone()));

    let auth = match connection.auth {
        ConnectionAuth::Password { password } => ImapAuth::LOGIN {
            username: connection.username,
            password,
        },
        ConnectionAuth::Xoauth2 { access_token, .. } => ImapAuth::XOAUTH2 {
            username: connection.username,
            access_token: access_token.secret.expose_secret().to_string(),
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

    listen_handle.abort();

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
            tracing::info!("Dropping interrupt for connection task as it is stopped");
            drop(interrupt);
            break;
        } else {
            tracing::debug!("Connection task is not stopped yet");
        }

        tokio::time::sleep(time::Duration::from_secs(10)).await;
    }
}
