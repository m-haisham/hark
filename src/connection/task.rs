use std::{fmt::Debug, sync::Arc};

use async_imap::{Session, types::Mailbox};
use chrono::{DateTime, Utc};
use eyre::{Context, eyre};
use stop_token::StopSource;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, watch},
};
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::{
        refresh::{get_refreshed_connection_from_store, refresh_connection_auth},
        types::{ConnectionEvent, ConnectionEventKind, ImapFlavour},
    },
    data::Data,
    imap::{
        ImapListenError,
        connect::{ImapAuth, ImapConnectionConfig},
        handle_idle_event, handle_idle_response,
        session::ImapSession,
    },
    settings::IdleSettings,
    window::EventWindow,
};

use super::types::{Connection, ConnectionAuth, ConnectionCommand, ConnectionId};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: ConnectionId,
    pub data: Arc<Data>,
    pub receiver: mpsc::Receiver<ConnectionCommand>,
    pub background: async_channel::Sender<BackgroundCommand>,
    pub idle_settings: IdleSettings,
}

#[instrument(name = "Listen for commands to connection", skip_all, fields(id = %id))]
async fn listen_command(
    mut receiver: mpsc::Receiver<ConnectionCommand>,
    id: ConnectionId,
    stop_tx: watch::Sender<bool>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ConnectionCommand::Stop => {
                let _ = stop_tx.send(true);
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
        idle_settings,
    } = task;

    background
        .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
            id: id.clone(),
            event: ConnectionEventKind::Starting,
        }))
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to send starting event to background task")?;

    let (stop_tx, stop_rx) = watch::channel(false);
    let listen_handle = tokio::spawn(listen_command(receiver, id.clone(), stop_tx));

    let mut error_window = EventWindow::new();

    loop {
        let connection = get_refreshed_connection_from_store(&data, &id)
            .await
            .wrap_err("Failed to get refreshed connection from data store")?;

        let stop_rx = stop_rx.clone();

        let connection_config = imap_connection_config(&connection);

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

        let mailbox = session.select(&connection.mailbox).await?;

        let result = match session {
            ImapSession::Tcp(session) => {
                idle(&id, connection, session, stop_rx, mailbox, &background).await
            }
            ImapSession::Tls(session) => {
                idle(&id, connection, session, stop_rx, mailbox, &background).await
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
                // FIXME: missing tests
                error_window.push(idle_settings.error_window);
                if error_window.exceeds_threshold(idle_settings.error_threshold) {
                    tracing::error!(
                        "Connection task exceeded error threshold, aborting with error: {:?}",
                        e
                    );
                    break;
                }

                tracing::error!("Retrying failed connection with error: {:?}", e);
                refresh_connection_auth(&data, &id)
                    .await
                    .wrap_err("Failed to refresh connection")?;
            }
        }
    }

    // FIXME: should abort the listen handle during errors
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
    stop_rx: watch::Receiver<bool>,
    mut mailbox: Mailbox,
    background_sender: &async_channel::Sender<BackgroundCommand>,
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

        let response_result = tokio::select! {
            result = idle_wait => result,
            _ = drop_interrupt_when_stopped(stop_rx.clone(), interrupt) => break,
            _ = terminate_on_expired(expires_at) => return Err(IdleError::Expired),
        };

        let response = match response_result {
            Ok(response) => response,
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
                ));
            }
        };

        let seq = handle_idle_event(&mut mailbox, result)
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to handle IDLE event")?;

        if let Some(sequence) = seq {
            background_sender
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::MessageSeq(sequence),
                }))
                .await
                .map_err(|e| eyre!(e))
                .wrap_err("Failed to send message sequence to background task")?;
        }
    }

    Ok(())
}

async fn drop_interrupt_when_stopped(mut stop_rx: watch::Receiver<bool>, interrupt: StopSource) {
    let _ = stop_rx.wait_for(|&stop| stop).await;
    tracing::info!("Dropping interrupt for connection task as it is stopped");
    drop(interrupt);
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
