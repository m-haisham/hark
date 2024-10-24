//! Represents a lazy session with an IMAP server.
//!
//! This module implements a lazy session with an IMAP server. The session is
//! lazy in the sense that it does not keep a connection open the entire time.
//! Instead, it opens a connection when needed and closes it after a certain
//! period of inactivity or when access token expires.
//!
//! The session is kept alive by sending NOOP commands to the server periodically.
//! The session however will not respond to any unsolicited responses from the server.
//!
//! The primary use case for this module is to fetch messages from an IMAP server
//! when we receive a sequence from the idle thread.
//!
//! This session will recieve commands from the idle thread and then fetch the messages
//! and push them to the background thread.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_channel::RecvError;
use eyre::{eyre, Context};
use tokio::sync::mpsc;
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::types::{ConnectionEvent, ConnectionEventKind, ConnectionId},
    imap::MessageParseResult,
};

use super::{connect::ImapConnectionConfig, ImapSession};

pub enum LazyCommand {
    Fetch { seq: String },
    Exit,
}

pub enum LazyEvent {
    Exit,
}

#[derive(Debug)]
pub struct ImapLazySession {
    connection_id: ConnectionId,
    state: Arc<Mutex<ImapLazyState>>,
    timeout: Duration,
    command_sender: async_channel::Sender<LazyCommand>,
    command_receiver: async_channel::Receiver<LazyCommand>,
    background_sender: async_channel::Sender<BackgroundCommand>,
    event_sender: mpsc::Sender<LazyEvent>,
}

#[derive(Debug)]
pub struct ImapLazyState {
    worker: Option<ImapLazyWorker>,
}

#[derive(Debug)]
pub struct ImapLazyWorker {
    connection_id: ConnectionId,
    session: ImapSession,
    command_receiver: async_channel::Receiver<LazyCommand>,
    event_sender: mpsc::Sender<LazyEvent>,
    background_sender: async_channel::Sender<BackgroundCommand>,
    timeout: Duration,
}

impl ImapLazySession {
    pub fn new(
        connection_id: ConnectionId,
        timeout: Duration,
        background_sender: async_channel::Sender<BackgroundCommand>,
    ) -> Self {
        let (command_sender, command_receiver) = async_channel::bounded(1024);
        let (event_sender, event_receiver) = mpsc::channel(1024);

        let state = Arc::new(Mutex::new(ImapLazyState { worker: None }));
        tokio::spawn(event_listener(Arc::clone(&state), event_receiver));

        Self {
            connection_id,
            timeout,
            command_sender,
            command_receiver,
            event_sender,
            background_sender,
            state,
        }
    }

    pub async fn start(&mut self, config: &ImapConnectionConfig) -> eyre::Result<()> {
        let session = ImapSession::connect(config).await?;

        let worker = ImapLazyWorker {
            connection_id: self.connection_id.clone(),
            session,
            command_receiver: self.command_receiver.clone(),
            event_sender: self.event_sender.clone(),
            background_sender: self.background_sender.clone(),
            timeout: self.timeout,
        };

        let mut state = self
            .state
            .lock()
            .map_err(|e| eyre!(e.to_string())) // FIXME: cant recover from this error
            .wrap_err("Failed to acquire lock")?;

        state.worker = Some(worker);

        Ok(())
    }

    pub async fn worker_is_running(&self) -> eyre::Result<bool> {
        let lock = self
            .state
            .lock()
            .map_err(|e| eyre!(e.to_string())) // FIXME: cant recover from this error
            .wrap_err("Failed to acquire lock")?;

        Ok(lock.worker.is_some())
    }

    pub async fn send(
        &mut self,
        config: &ImapConnectionConfig,
        command: LazyCommand,
    ) -> eyre::Result<()> {
        if !self.worker_is_running().await? {
            self.start(config).await?;
        }

        self.command_sender.try_send(command)?;

        Ok(())
    }
}

#[instrument(skip_all)]
async fn event_listener(
    state: Arc<Mutex<ImapLazyState>>,
    mut event_receiver: mpsc::Receiver<LazyEvent>,
) {
    loop {
        match event_receiver.recv().await {
            Some(LazyEvent::Exit) => match state.lock() {
                Ok(mut state) => {
                    state.worker = None;
                    break;
                }
                Err(e) => {
                    tracing::error!("Failed to acquire lock: {:?}", e);
                    break;
                }
            },
            None => break,
        }
    }
}

#[instrument(skip_all)]
pub async fn lazy_worker(worker: ImapLazyWorker) {
    let mut session = worker.session;
    let mut interval = tokio::time::interval(worker.timeout);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                tracing::debug!("Sending NOOP command to keep session alive");
                if let Err(e) = session.noop().await {
                    tracing::error!("Failed to send NOOP command: {:?}", e);
                    break;
                }
            }
            command = worker.command_receiver.recv() => {
                match command {
                    Ok(LazyCommand::Fetch { seq }) => {
                        tracing::debug!("Fetching messages with sequence: {}", seq);

                        let messages = match session.fetch_messages(&seq).await {
                            Ok(messages) => messages,
                            Err(e) => {
                                tracing::error!("Failed to fetch messages: {:?}", e);
                                continue;
                            }
                        };

                        let messages = messages.into_iter().flat_map(|e| match e {
                            MessageParseResult::Message(message) => Some(message),
                            _ => None, // FIXME: should handle this.
                        });

                        for message in messages {
                            if let Err(e) = worker
                                .background_sender
                                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                                    id: worker.connection_id.clone(),
                                    event: ConnectionEventKind::MessageReceived(message),
                                }))
                                .await
                            {
                                tracing::error!("Failed to send message to background: {:?}", e);
                                break;
                            }
                        }
                    }
                    Ok(LazyCommand::Exit) => {
                        tracing::debug!("Received exit command");
                        break;
                    }
                    Err(RecvError) => {
                        tracing::debug!("Command channel closed");
                        break;
                    }
                }
            }
        }
    }

    if let Err(e) = worker.event_sender.send(LazyEvent::Exit).await {
        tracing::error!("Failed to send exit event: {:?}", e);
    }
}
