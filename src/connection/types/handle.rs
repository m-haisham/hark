use serde::Serialize;
use tokio::sync::mpsc::{self, error::SendError};

use super::{Connection, ConnectionCommand, ConnectionId, ConnectionState};

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub connection: Connection,
    #[serde(skip)]
    sender: mpsc::Sender<ConnectionCommand>,
}

impl ConnectionHandle {
    pub fn new(
        id: ConnectionId,
        connection: Connection,
        sender: mpsc::Sender<ConnectionCommand>,
    ) -> Self {
        Self {
            id,
            state: ConnectionState::Starting,
            connection,
            sender,
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
    }
}
