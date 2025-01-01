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

use std::sync::{Arc, Mutex};

use async_channel::RecvError;
use eyre::{eyre, Context};
use tokio::{sync::mpsc, task::JoinHandle, time::MissedTickBehavior};
use tracing::{instrument, Span};

use crate::{
    background::command::{BackgroundCommand, SessionEvent},
    connection::{
        refresh::get_refreshed_connection_from_store,
        task::imap_connection_config,
        types::{ConnectionEvent, ConnectionEventKind, ConnectionId},
    },
    data::Data,
    imap::{session::ImapSession, MessageParseResult},
    settings::LazySettings,
};

#[derive(Debug)]
pub enum LazyCommand {
    FetchSequence(String),
    Exit,
}

#[derive(Debug)]
pub enum LazyEvent {
    Exit,
}

#[derive(Debug)]
pub struct ImapLazySession {
    pub connection_id: ConnectionId,
    pub data: Arc<Data>,
    pub state: Arc<Mutex<ImapLazyState>>,
    pub command_sender: async_channel::Sender<LazyCommand>,
    pub command_receiver: async_channel::Receiver<LazyCommand>,
    pub background_sender: async_channel::Sender<BackgroundCommand>,
    pub event_sender: mpsc::Sender<LazyEvent>,
    pub settings: LazySettings,
}

#[derive(Debug)]
pub struct ImapLazyState {
    worker: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub struct ImapLazyWorkerState {
    connection_id: ConnectionId,
    session: ImapSession,
    command_receiver: async_channel::Receiver<LazyCommand>,
    event_sender: mpsc::Sender<LazyEvent>,
    background_sender: async_channel::Sender<BackgroundCommand>,
    settings: LazySettings,
}

impl ImapLazySession {
    pub fn new(
        connection_id: ConnectionId,
        data: Arc<Data>,
        background_sender: async_channel::Sender<BackgroundCommand>,
        settings: LazySettings,
    ) -> Self {
        let (command_sender, command_receiver) = async_channel::bounded(1024);
        let (event_sender, event_receiver) = mpsc::channel(1024);

        let state = Arc::new(Mutex::new(ImapLazyState { worker: None }));

        tokio::spawn(event_listener(
            connection_id.clone(),
            Arc::clone(&state),
            event_receiver,
            command_receiver.clone(),
            background_sender.clone(),
        ));

        Self {
            connection_id,
            data,
            command_sender,
            command_receiver,
            event_sender,
            background_sender,
            state,
            settings,
        }
    }

    #[instrument(
        skip_all,
        fields(
            connection_id = %self.connection_id,
            mailbox,
        ),
    )]
    pub async fn start(&self) -> eyre::Result<()> {
        let connection = get_refreshed_connection_from_store(&self.data, &self.connection_id)
            .await
            .wrap_err("Failed to get refreshed connection from store")?;

        let connection_config = imap_connection_config(&connection);
        let mut session = ImapSession::connect(&connection_config).await?;

        session
            .select(&connection.mailbox)
            .await
            .wrap_err("Failed to select mailbox while lazy session start")?;

        let worker = ImapLazyWorkerState {
            connection_id: self.connection_id.clone(),
            session,
            command_receiver: self.command_receiver.clone(),
            event_sender: self.event_sender.clone(),
            background_sender: self.background_sender.clone(),
            settings: self.settings.clone(),
        };

        let handle = tokio::spawn(lazy_worker(worker));

        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| eyre!(e.to_string())) // FIXME: cant recover from this error
                .wrap_err("Failed to acquire lock")?;

            state.worker = Some(handle);
        }

        self.background_sender
            .send(BackgroundCommand::SessionEvent(SessionEvent::Started(
                self.connection_id.clone(),
            )))
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to send session started event")?;

        Ok(())
    }

    pub async fn is_running(&self) -> eyre::Result<bool> {
        let lock = self
            .state
            .lock()
            .map_err(|e| eyre!(e.to_string())) // FIXME: cant recover from this error
            .wrap_err("Failed to acquire lock")?;

        Ok(lock.worker.is_some())
    }

    #[instrument(
        skip_all,
        fields(
            connection_id = %self.connection_id,
            command = ?command,
            worker_is_running = tracing::field::Empty,
        ),
    )]
    pub async fn send(&self, command: LazyCommand) -> eyre::Result<()> {
        if !self.is_running().await? {
            Span::current().record("worker_is_running", &false);
            tracing::info!("Lazy worker is not running, starting worker");
            self.start().await?;
        } else {
            Span::current().record("worker_is_running", &true);
        }

        self.command_sender
            .try_send(command)
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to send command to lazy worker")?;

        Ok(())
    }

    pub async fn stop(&mut self) -> eyre::Result<()> {
        if self.is_running().await? {
            // We can ignore the error here this means the worker has already exited
            let _ = self.command_sender.send(LazyCommand::Exit).await;
        }

        Ok(())
    }

    pub async fn wait_for_exit(&self) {
        let mut state = self.state.lock().expect("Failed to acquire lock");

        if let Some(worker) = state.worker.take() {
            drop(state); // Release the lock before waiting for the worker

            tracing::info!("Waiting for lazy worker to exit");
            if let Err(e) = worker.await {
                tracing::error!("Lazy worker exited with error: {:?}", e);
            }
        } else {
            tracing::debug!("Lazy worker is not running, nothing to wait for");
        }
    }
}

#[instrument(skip_all, fields(connection_id = %connection_id))]
async fn event_listener(
    connection_id: ConnectionId,
    state: Arc<Mutex<ImapLazyState>>,
    mut event_receiver: mpsc::Receiver<LazyEvent>,
    command_receiver: async_channel::Receiver<LazyCommand>,
    background_sender: async_channel::Sender<BackgroundCommand>,
) {
    loop {
        match event_receiver.recv().await {
            Some(LazyEvent::Exit) => {
                tracing::info!("Received exit event in lazy session");

                match state.lock() {
                    Ok(mut state) => {
                        state.worker = None;
                    }
                    Err(e) => {
                        // FIXME: this is a critical error, we should replace the whole session in the pool
                        // and drain the command channel to the new session
                        tracing::error!("Failed to acquire lock: {:?}", e);
                        break;
                    }
                }

                if command_receiver.is_empty() {
                    let result = background_sender
                        .send(BackgroundCommand::SessionEvent(SessionEvent::Exited(
                            connection_id.clone(),
                        )))
                        .await
                        .map_err(|e| eyre!(e))
                        .wrap_err("Failed to send session closed event");

                    if let Err(e) = result {
                        tracing::error!("{:?}", e);
                    }
                } else {
                    tracing::info!("More commands in queue, restarting session after exit");

                    let result = background_sender
                        .send(BackgroundCommand::RestartSession(connection_id.clone()))
                        .await
                        .map_err(|e| eyre!(e))
                        .wrap_err("Failed to send restart session command");

                    // The only scenario this can happen is if background thread is stopped. In that case
                    // no point in trying to start the session again. Hence, why this is a warning.
                    if let Err(e) = result {
                        tracing::warn!("{:?}", e);
                    }
                }
            }
            None => {
                tracing::info!("Event channel closed, exiting lazy session");

                let result = background_sender
                    .send(BackgroundCommand::SessionEvent(SessionEvent::Exited(
                        connection_id.clone(),
                    )))
                    .await
                    .map_err(|e| eyre!(e))
                    .wrap_err("Failed to send session closed event");

                if let Err(e) = result {
                    tracing::error!("{:?}", e);
                }

                break;
            }
        }
    }
}

#[instrument(skip_all)]
pub async fn lazy_worker(worker: ImapLazyWorkerState) {
    let mut session = worker.session;
    let mut heartbeat = tokio::time::interval(worker.settings.heartbeat);

    // The only scenario where we would skip a tick is when fetching messages
    // since that also counts as a heartbeat, we can skip the tick
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut fetch_count = 0;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                tracing::debug!("Sending NOOP command to keep session alive");
                if let Err(e) = session.noop().await {
                    tracing::error!("Failed to send NOOP command: {:?}", e);
                    break;
                }
            }
            _ = tokio::time::sleep(worker.settings.timeout) => {
                tracing::info!("Session timed out, logging out");
                if let Err(e) = session.logout().await {
                    tracing::error!("Failed to logout from session after timeout: {e:?}");
                }
                break;
            }
            command = worker.command_receiver.recv() => {
                match command {
                    Ok(LazyCommand::FetchSequence(seq)) => {
                        tracing::info!("Fetching messages with sequence: {}", seq);

                        let messages = match session.fetch_messages(&seq).await {
                            Ok(messages) => messages,
                            Err(async_imap::error::Error::Io(e)) => {
                                tracing::error!("IO error while fetching messages: {:?}", e);
                                // FIXME: this command is lost possibly should be retried
                                break;
                            }
                            Err(async_imap::error::Error::No(e)) => {
                                tracing::error!("NO response while fetching messages: {:?}", e);
                                // FIXME: this command is lost possibly should be retried
                                break;
                            }
                            Err(async_imap::error::Error::ConnectionLost) => {
                                tracing::error!("Connection lost while fetching messages");
                                // FIXME: this command is lost possibly should be retried
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch messages: {:?}", e);
                                // FIXME: this command is lost possibly should be retried
                                continue;
                            }
                        };

                        let messages = messages.into_iter().flat_map(|e| match e {
                            MessageParseResult::Message(message) => Some(message),
                            _ => None, // FIXME: should handle this.
                        }).collect::<Vec<_>>();

                        tracing::debug!("Sending messages to background, count: {}", messages.len());

                        for message in messages {
                            if let Err(e) = worker
                                .background_sender
                                .send(BackgroundCommand::ConnectionEvent(ConnectionEvent {
                                    id: worker.connection_id.clone(),
                                    event: ConnectionEventKind::MessageParsed(message),
                                }))
                                .await
                            {
                                tracing::error!("Failed to send message to background: {:?}", e);
                                break;
                            }
                        }

                        fetch_count += 1;

                        if let Some(max_fetch_count) = worker.settings.max_fetch_count {
                            if fetch_count >= max_fetch_count {
                                tracing::info!("Reached max fetch count, logging out");
                                if let Err(e) = session.logout().await {
                                    tracing::error!("Failed to logout from session after max fetch count: {e:?}");
                                }
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
        tracing::error!("Failed to send lazy exit event: {e:?}");
    }
}
