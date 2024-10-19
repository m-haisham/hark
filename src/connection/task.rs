use std::{
    fmt::Debug,
    sync::Arc,
    time::{self, Duration},
};

use async_imap::Session;
use chrono::{DateTime, TimeDelta, Utc};
use eyre::{eyre, Context};
use futures::lock::Mutex;
use oauth2::TokenResponse;
use stop_token::StopSource;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::types::{ConnectionEvent, ConnectionEventKind, ImapFlavour, OAuth2},
    imap::{
        handle_idle_event, handle_idle_response, imap_connect_tcp, imap_connect_tls, imap_listen,
        ImapAuth, ImapConnectionConfig, ImapListenConfig, ImapListenError,
    },
};

use super::types::{Connection, ConnectionAuth, ConnectionCommand, ConnectionId};

#[derive(Debug)]
pub struct ConnectionTask {
    pub id: ConnectionId,
    pub connection: Connection,
    pub receiver: mpsc::Receiver<ConnectionCommand>,
    pub background: async_channel::Sender<BackgroundCommand>,
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
        mut connection,
        receiver,
        background,
    } = task;

    background
        .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
            id: id.clone(),
            event: ConnectionEventKind::Starting,
        }))
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to send starting event to background task")?;

    let state = Arc::new(Mutex::new(ConnectionTaskState { stop: false }));
    let listen_handle = tokio::spawn(listen_command(receiver, id.clone(), state.clone()));

    if let ConnectionAuth::OAuth2(oauth2) = connection.auth {
        // Update the access token if it is about to expire, expired, or expires_at is not provided
        if (oauth2.expires_at.unwrap_or_else(Utc::now) - Utc::now()).num_seconds() < 60 {
            connection.auth = ConnectionAuth::OAuth2(refresh_access_token(&id, oauth2).await?);

            background
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::Updated(connection.clone()),
                }))
                .await?;
        } else {
            connection.auth = ConnectionAuth::OAuth2(oauth2);
        }
    }

    loop {
        let inner_connection = connection.clone();
        let state = Arc::clone(&state);

        let imap_connection = imap_connection_config(&inner_connection).await?;

        let result = if inner_connection.tls {
            tracing::info!("Connecting to IMAP server with TLS");

            let session = imap_connect_tls(&imap_connection)
                .await
                .map_err(|e| eyre::eyre!(e))
                .wrap_err("Failed to connect to IMAP server")?;

            // TODO: handle unsolicited responses

            background
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::Started,
                }))
                .await?;

            idle(&id, inner_connection, session, background.clone(), state).await
        } else {
            tracing::info!("Connecting to IMAP server without TLS");

            let session = imap_connect_tcp(&imap_connection)
                .await
                .map_err(|e| eyre::eyre!(e))
                .wrap_err("Failed to connect to IMAP server")?;

            // TODO: handle unsolicited responses

            background
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::Started,
                }))
                .await?;

            idle(&id, inner_connection, session, background.clone(), state).await
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

                if let ConnectionAuth::OAuth2(oauth2) = connection.auth {
                    connection.auth =
                        ConnectionAuth::OAuth2(refresh_access_token(&id, oauth2).await?);
                }
            }
            Err(e) => {
                tracing::error!("Retrying failed connection with error: {:?}", e);
                // TODO: Add retry with backoff strategy, e.g.:
                // maxmimum retry count with in a time period (e.g. 5 times in 1 hour)

                if let ConnectionAuth::OAuth2(oauth2) = connection.auth {
                    connection.auth =
                        ConnectionAuth::OAuth2(refresh_access_token(&id, oauth2).await?);
                }
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

pub async fn imap_connection_config(connection: &Connection) -> eyre::Result<ImapConnectionConfig> {
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
        auth,
        flavour,
    };

    Ok(imap_connection)
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
    session: Session<T>,
    background: async_channel::Sender<BackgroundCommand>,
    state: Arc<Mutex<ConnectionTaskState>>,
) -> Result<(), IdleError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let listen_config = ImapListenConfig {
        mailbox: connection.mailbox,
        lookback_duration: Some(TimeDelta::try_days(30).unwrap()),
    };

    let (mut session, mut listen) = imap_listen(session, listen_config)
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to start listening to IMAP server")?;

    let expires_at = match connection.auth {
        ConnectionAuth::OAuth2(oauth2) => oauth2.expires_at,
        ConnectionAuth::Password { .. } => None,
    };

    loop {
        let mut idle = session.idle();
        idle.init()
            .await
            .map_err(|e| eyre::eyre!(e))
            .wrap_err("Failed to initialise IDLE")?;

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

        let (idle, result) = match handle_idle_response(idle, response).await {
            Ok(v) => v,
            Err(ImapListenError::Exit) => break,
            Err(ImapListenError::Imap(e)) => {
                return Err(IdleError::Other(
                    eyre!(e).wrap_err("Error while handling response"),
                ))
            }
        };

        session = idle
            .done()
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to complete IDLE")?;

        let messages = handle_idle_event(&mut session, &mut listen, result)
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to handle IDLE event")?;

        for message in messages {
            tracing::debug!("Received message: {:?}", message);

            background
                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                    id: id.clone(),
                    event: ConnectionEventKind::MessageReceived(message),
                }))
                .await
                .map_err(|e| eyre::eyre!(e))
                .wrap_err("Failed to send message to background task")?;
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

#[instrument(
    name = "Refresh access token",
    skip_all,
    fields(connection_id = %connection_id)
)]
pub async fn refresh_access_token(
    connection_id: &ConnectionId,
    oauth2: OAuth2,
) -> eyre::Result<OAuth2> {
    let OAuth2 {
        refresh_token,
        config,
        ..
    } = oauth2;

    tracing::info!("Refreshing access token for connection");

    let client: oauth2::basic::BasicClient = oauth2::Client::new(
        config.client_id.clone(),
        Some(config.client_secret.clone()),
        config.auth_uri.clone(),
        Some(config.token_uri.clone()),
    );

    let request = client.exchange_refresh_token(&refresh_token);

    tracing::debug!("Requesting new access token");

    let response = request
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to refresh access token")?;

    let expires_in = response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));

    let expires_at = chrono::Utc::now() + chrono::Duration::from_std(expires_in)?
        - chrono::Duration::seconds(60); // subtract 60 seconds to be safe

    // Use the refresh token from the response if provided, otherwise use the existing one
    // This is to handle the case where the refresh token is rotated
    let refresh_token = response.refresh_token().cloned().unwrap_or(refresh_token);

    let oauth2 = OAuth2 {
        access_token: response.access_token().clone(),
        expires_at: Some(expires_at),
        refresh_token,
        config,
    };

    Ok(oauth2)
}
