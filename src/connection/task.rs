use std::{
    fmt::Debug,
    sync::Arc,
    time::{self},
};

use async_imap::{types::Mailbox, Session};
use chrono::{DateTime, Utc};
use eyre::{eyre, Context};
use stop_token::StopSource;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, Mutex},
};
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::{
        refresh::{
            get_connection_from_store, is_connection_auth_refresh_needed, refresh_connection_auth,
        },
        types::{ConnectionEvent, ConnectionEventKind, ImapFlavour},
    },
    data::Data,
    imap::{
        connect::{ImapAuth, ImapConnectionConfig},
        handle_idle_event, handle_idle_response,
        lazy::{ImapLazySession, LazyCommand},
        ImapListenError, ImapSession,
    },
};

use super::types::{Connection, ConnectionAuth, ConnectionCommand, ConnectionId};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: ConnectionId,
    pub data: Arc<Data>,
    pub receiver: mpsc::Receiver<ConnectionCommand>,
    pub background: async_channel::Sender<BackgroundCommand>,
    pub lazy: Arc<Mutex<ImapLazySession>>,
}

#[derive(Debug)]
pub struct ConnectionTaskState {
    pub stop: bool,
    pub lazy: Arc<Mutex<ImapLazySession>>,
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
                tracing::info!("Connection received stop command");
                return;
            }
        }
    }
}

#[tracing::instrument(name = "Connection Task", skip(task), fields(id = %task.id))]
pub async fn run_connection_task(task: ConnectionTask) {
    let id = task.id.clone();
    let background = task.background.clone();

    let result = run_connection_task_inner(task).await;

    if let Err(err) = result {
        tracing::error!("Connection task failed: {:?}", err);

        let command_result = background
            .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                id: id.clone(),
                event: ConnectionEventKind::Failed(format!("{:?}", err)),
            }))
            .await;

        if let Err(err) = command_result {
            tracing::error!("Failed to send failed event to background task: {:?}", err);
        }
    }
}

pub async fn run_connection_task_inner(task: ConnectionTask) -> eyre::Result<()> {
    let ConnectionTask {
        id,
        data,
        receiver,
        background,
        lazy,
    } = task;

    background
        .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
            id: id.clone(),
            event: ConnectionEventKind::Starting,
        }))
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to send starting event to background task")?;

    // FIXME: Can be optimized to only use Arc<Mutex<_>> for the stop flag (mutable part)
    let state = Arc::new(Mutex::new(ConnectionTaskState {
        stop: false,
        lazy: lazy.clone(),
    }));

    let listen_handle = tokio::spawn(listen_command(receiver, id.clone(), state.clone()));

    let connection = get_connection_from_store(&data, &id)
        .await
        .wrap_err("Failed to get connection from data store")?;

    if is_connection_auth_refresh_needed(&connection).await {
        refresh_connection_auth(&data, &id)
            .await
            .wrap_err("Failed to refresh connection")?;
    }

    loop {
        let inner_connection = connection.clone();
        let state = Arc::clone(&state);

        let connection_config = imap_connection_config(&inner_connection);

        let mut session = ImapSession::connect(&connection_config)
            .await
            .map_err(|e| eyre::eyre!(e))
            .wrap_err("Failed to connect to IMAP server")?;

        background
            .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                id: id.clone(),
                event: ConnectionEventKind::Started,
            }))
            .await?;

        if !session.has_idle_capability().await? {
            tracing::error!("IMAP server does not support IDLE command");
            background
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::Failed(
                        "IMAP server does not support IDLE command".to_string(),
                    ),
                }))
                .await?;

            return Ok(());
        }

        let mailbox = session.select(&inner_connection.mailbox).await?;

        let result = match session {
            ImapSession::Tcp(session) => {
                idle(&id, inner_connection, session, &lazy, state, mailbox).await
            }
            ImapSession::Tls(session) => {
                idle(&id, inner_connection, session, &lazy, state, mailbox).await
            }
        };

        match result {
            Ok(()) => {
                tracing::info!("Connection task completed successfully");
                break;
            }
            Err(IdleError::Expired) => {
                tracing::info!(
                    "Connection task terminated due to expired access token, refreshing token."
                );

                refresh_connection_auth(&data, &id)
                    .await
                    .wrap_err("Failed to refresh connection")?;
            }
            Err(e) => {
                tracing::error!("Retrying failed connection with error: {:?}", e);
                // TODO: Add retry with backoff strategy, e.g.:
                // maxmimum retry count with in a time period (e.g. 5 times in 1 hour)

                refresh_connection_auth(&data, &id)
                    .await
                    .wrap_err("Failed to refresh connection")?;
            }
        }
    }

    tracing::debug!("Aborting command listening task");
    listen_handle.abort();

    tracing::debug!("Sending stopped event to background task");
    background
        .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
            id,
            event: ConnectionEventKind::Stopped,
        }))
        .await?;

    Ok(())
}

pub fn imap_connection_config(connection: &Connection) -> ImapConnectionConfig {
    let auth = match &connection.auth {
        ConnectionAuth::Password { password } => ImapAuth::LOGIN {
            username: connection.username.clone(),
            password: password.clone(),
        },
        ConnectionAuth::OAuth2(oauth2) => ImapAuth::XOAUTH2 {
            username: connection.username.clone(),
            access_token: oauth2.access_token.clone(),
        },
    };

    let mut flavour = connection.flavour;
    if flavour.is_none() {
        flavour = ImapFlavour::from_host(&connection.host);
        if let Some(flavour) = flavour.as_ref() {
            tracing::debug!("Detected IMAP flavour: {:?}", flavour);
        }
    }

    let imap_connection = ImapConnectionConfig {
        host: connection.host.clone(),
        port: connection.port,
        tls: connection.tls,
        auth,
        flavour,
    };

    imap_connection
}

#[derive(Debug, thiserror::Error)]
pub enum IdleError {
    #[error("Connection terminated after access token expired or revoked")]
    Expired,
    #[error("{0}")]
    Other(#[from] eyre::Error),
}

#[tracing::instrument(name = "IMAP Idle", skip_all, fields(id = %id))]
async fn idle<T>(
    id: &ConnectionId,
    connection: Connection,
    mut session: Session<T>,
    lazy: &Arc<Mutex<ImapLazySession>>,
    state: Arc<Mutex<ConnectionTaskState>>,
    mut mailbox: Mailbox,
) -> Result<(), IdleError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let expires_at = match &connection.auth {
        ConnectionAuth::OAuth2(oauth2) => oauth2.expires_at,
        ConnectionAuth::Password { .. } => None,
    };

    let mut idle = session.idle();

    idle.init()
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to initialise IDLE")?;

    loop {
        let (idle_wait, interrupt) = idle.wait();

        // check state for stop every 10 seconds and drop interrupt if stop is true
        let response_result = tokio::select! {
            result = idle_wait => result,
            _ = drop_interrupt_when_stopped(state.clone(), interrupt) => break,
            _ = terminate_on_expired(expires_at) => return Err(IdleError::Expired),
        };

        let response = match response_result {
            Ok(response) => response,
            // If its an IO error and the error message contains "DONE", it means the connection
            // was terminated by the server when access token expired or revoked
            Err(e) => return Err(IdleError::Other(eyre!(e).wrap_err("Error during IDLE"))),
        };

        let result = match handle_idle_response(response).await {
            Ok(v) => v,
            Err(ImapListenError::Exit) => {
                tracing::info!("Connection terminated gracefully, completing IDLE and logging out");

                session = idle
                    .done()
                    .await
                    .map_err(|e| eyre!(e))
                    .wrap_err("Failed to complete IDLE")?;

                session
                    .logout()
                    .await
                    .map_err(|e| eyre!(e))
                    .wrap_err("Failed to logout")?;

                break;
            }
            Err(ImapListenError::ResponseIgnored) => continue,
            Err(ImapListenError::Imap(e)) => {
                return Err(IdleError::Other(
                    eyre!(e).wrap_err("Error while handling response"),
                ))
            }
        };

        let seq = handle_idle_event(&mut mailbox, result)
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to handle IDLE event")?;

        if let Some(sequence) = seq {
            let lazy = lazy.lock().await;
            let command = LazyCommand::FetchSequence(sequence);
            lazy.send(command)
                .await
                .map_err(|e| eyre!(e))
                .wrap_err("Failed to send command to lazy worker")?;
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
            let lock = state.lock().await;
            lock.stop
        };

        if stop {
            tracing::info!("Dropping interrupt for connection task as it is stopped");
            drop(interrupt);
            break;
        } else {
            tracing::debug!("Connection is active, waiting for stop command");
        }

        tokio::time::sleep(time::Duration::from_secs(10)).await;
    }
}

#[instrument(
    name = "Terminate on expired",
    skip_all,
    fields(expires_at = ?expires_at)
)]
async fn terminate_on_expired(expires_at: Option<DateTime<Utc>>) {
    let duration = expires_at
        .map(|expires_at| {
            let duration = expires_at - Utc::now();
            duration.to_std().ok()
        })
        .flatten();

    if let Some(duration) = duration {
        tokio::time::sleep(duration).await;
    } else {
        futures::future::pending::<()>().await;
    }
}
