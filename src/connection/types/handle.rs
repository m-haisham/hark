use super::{Connection, ConnectionCommand, ConnectionId, ConnectionState};
use crate::{connection::refresh::get_connection_from_store, data::Data};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc::{self, error::SendError};

#[derive(Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    pub state: ConnectionState,
    data: Arc<Data>,
    sender: mpsc::Sender<ConnectionCommand>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub connection: Connection,
}

impl ConnectionHandle {
    pub fn new(id: ConnectionId, data: Arc<Data>, sender: mpsc::Sender<ConnectionCommand>) -> Self {
        Self {
            id,
            state: ConnectionState::Starting,
            data,
            sender,
        }
    }

    pub async fn info(&self) -> eyre::Result<ConnectionInfo> {
        Ok(ConnectionInfo {
            id: self.id.clone(),
            state: self.state.clone(),
            connection: get_connection_from_store(&self.data, &self.id).await?,
        })
    }

    pub async fn send(
        &self,
        command: ConnectionCommand,
    ) -> Result<(), SendError<ConnectionCommand>> {
        self.sender.send(command).await
    }

    pub async fn stop(&self) {
        // The task is already stopped if the receiver is dropped,
        // so we can ignore the error.
        let _ = self.send(ConnectionCommand::Stop).await;
    }
}
