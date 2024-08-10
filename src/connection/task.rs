use std::{
    fmt::Debug,
    sync::Arc,
    time::{self, Duration},
};

use anyhow::Context;
use async_imap::Session;
use chrono::{DateTime, TimeDelta, Utc};
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
    connection::types::{ConnectionEvent, ConnectionEventKind, OAuth2},
    imap::{
        handle_idle_event, handle_idle_response, imap_connect_tcp, imap_connect_tls, imap_listen,
        ImapAuth, ImapConnectionConfig, ImapListenConfig,
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
    let result = run_connection_task_inner(task).await;
    if let Err(err) = result {
        tracing::error!("Connection task failed: {:?}", err);
    }
}

pub async fn run_connection_task_inner(task: ConnectionTask) -> anyhow::Result<()> {
    let ConnectionTask {
        id,
        mut connection,
        receiver,
        background,
    } = task;

    let state = Arc::new(Mutex::new(ConnectionTaskState { stop: false }));
    let listen_handle = tokio::spawn(listen_command(receiver, id.clone(), state.clone()));

    if let ConnectionAuth::OAuth2(oauth2) = connection.auth {
        connection.auth = ConnectionAuth::OAuth2(refresh_access_token(&id, oauth2).await?);
    }

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

    let imap_connection = ImapConnectionConfig {
        host: connection.host.clone(),
        port: connection.port,
        auth,
        flavour: connection.flavour,
    };

    // TODO: add a loop here to reconnect if:
    // 1. connection is terminated when expired
    // 2. server closed connection unexpectedly

    // Add backoff strategy, e.g.:
    // maxmimum retry count with in a time period (e.g. 5 times in 1 hour)

    if connection.tls {
        tracing::info!("Connecting to IMAP server with TLS");

        let session = imap_connect_tls(&imap_connection)
            .await
            .context("Failed to connect to IMAP server")?;

        background
            .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                id: id.clone(),
                event: ConnectionEventKind::Started,
            }))
            .await?;

        idle(connection, session, state).await?;
    } else {
        tracing::info!("Connecting to IMAP server without TLS");

        let session = imap_connect_tcp(&imap_connection)
            .await
            .context("Failed to connect to IMAP server")?;

        background
            .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                id: id.clone(),
                event: ConnectionEventKind::Started,
            }))
            .await?;

        idle(connection, session, state).await?;
    }

    listen_handle.abort();

    background
        .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
            id,
            event: ConnectionEventKind::Stopped,
        }))
        .await?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum IdleError {
    #[error("Connection terminated after set timer")]
    Expired,
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

async fn idle<T>(
    connection: Connection,
    session: Session<T>,
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
        .context("Failed to start listening to IMAP server")?;

    let expires_at = match &connection.auth {
        ConnectionAuth::Password { .. } => None,
        ConnectionAuth::OAuth2(oauth2) => oauth2.expires_at,
    };

    loop {
        let mut idle = session.idle();
        idle.init().await.context("Failed to initialise IDLE")?;

        let (idle_wait, interrupt) = idle.wait();

        // check state for stop every 10 seconds and drop interrupt if stop is true
        // while simultaneously waiting for idle_wait to complete
        let response = tokio::select! {
            result = idle_wait => {
                result.context("Failed to wait for idle response")?
            },
            _ = drop_interrupt_when_stopped(state.clone(), interrupt) => break,
            _ = terminate_on_expired(expires_at) => return Err(IdleError::Expired),
        };

        let (idle, result) = handle_idle_response(idle, response)
            .await
            .context("Failed to handle IDLE response")?;

        session = idle.done().await.context("Failed to complete IDLE")?;

        let messages = handle_idle_event(&mut session, &mut listen, result)
            .await
            .context("Failed to handle IDLE event")?;

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
            tracing::info!("Dropping interrupt for connection task as it is stopped");
            drop(interrupt);
            break;
        } else {
            tracing::debug!("Connection task is not stopped yet");
        }

        tokio::time::sleep(time::Duration::from_secs(10)).await;
    }
}

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
) -> anyhow::Result<OAuth2> {
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
        .context("Failed to refresh access token")?;

    let expires_in = response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));

    let expires_at = chrono::Utc::now() + chrono::Duration::from_std(expires_in)?
        - chrono::Duration::seconds(60); // subtract 60 seconds to be safe

    let oauth2 = OAuth2 {
        access_token: response.access_token().clone(),
        expires_at: Some(expires_at),
        refresh_token,
        config,
    };

    Ok(oauth2)
}
