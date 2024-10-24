use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{
    mpsc::{self, error::SendError},
    Mutex,
};

use crate::imap::lazy::ImapLazySession;

use super::{Connection, ConnectionCommand, ConnectionId, ConnectionState};

#[derive(Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub connection: Connection,
    sender: mpsc::Sender<ConnectionCommand>,
    lazy: Arc<Mutex<ImapLazySession>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub connection: Connection,
}

impl ConnectionHandle {
    pub fn new(
        id: ConnectionId,
        connection: Connection,
        sender: mpsc::Sender<ConnectionCommand>,
        lazy: Arc<Mutex<ImapLazySession>>,
    ) -> Self {
        Self {
            id,
            state: ConnectionState::Starting,
            connection,
            sender,
            lazy,
        }
    }

    pub fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: self.id.clone(),
            state: self.state.clone(),
            connection: self.connection.clone(),
        }
    }

    pub async fn send(
        &self,
        command: ConnectionCommand,
    ) -> Result<(), SendError<ConnectionCommand>> {
        self.sender.send(command).await
    }

    pub async fn stop(&mut self) {
        // The task is already stopped if the receiver is dropped,
        // so we can ignore the error.
        let _ = self.send(ConnectionCommand::Stop).await;

        // FIXME: This error should not be ignored
        let mut lazy = self.lazy.lock().await;
        let _ = lazy.stop().await;
    }

    pub async fn wait_for_exit(&mut self) {
        let lazy = self.lazy.lock().await;
        let _ = lazy.wait_for_exit().await;
    }
}
