use std::collections::HashMap;

use tokio::task::JoinHandle;
use tracing::instrument;
use types::{Connection, ConnectionCommand, ConnectionHandle, ConnectionId};

use crate::background::{self, command::BackgroundCommand};

pub mod task;
pub mod types;

#[derive(Debug)]
pub struct ConnectionPool {
    pool: HashMap<ConnectionId, ConnectionHandle>,
    handles: HashMap<ConnectionId, JoinHandle<()>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            pool: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    #[instrument(name = "Spawn Connection Task", skip(self, connection))]
    pub fn spawn(
        &mut self,
        id: ConnectionId,
        connection: Connection,
        background: async_channel::Sender<BackgroundCommand>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);

        let task = task::ConnectionTask {
            id: id.clone(),
            connection: connection.clone(),
            receiver,
            background,
        };

        let handle = ConnectionHandle::new(id.clone(), connection, sender);
        self.pool.insert(id.clone(), handle);

        let join_handle = tokio::spawn(task::run_connection_task(task));
        self.handles.insert(id, join_handle);
    }

    pub fn list_connections(&self) -> impl Iterator<Item = (&ConnectionId, &ConnectionHandle)> {
        self.pool.iter()
    }

    pub fn get_connection(&self, id: &ConnectionId) -> Option<&ConnectionHandle> {
        self.pool.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &ConnectionId) -> Option<&mut ConnectionHandle> {
        self.pool.get_mut(id)
    }

    pub fn remove_connection(&mut self, id: &ConnectionId) -> Option<ConnectionHandle> {
        self.pool.remove(id)
    }

    pub async fn stop_all(&mut self) {
        for (_, handle) in self.pool.iter() {
            let _ = handle.send(ConnectionCommand::Stop).await;
        }
    }

    pub async fn join_all(&mut self) {
        let mut handles = Vec::new();
        for (_, handle) in self.handles.drain() {
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    pub async fn remove_join(&mut self, id: &ConnectionId) -> Option<JoinHandle<()>> {
        self.handles.remove(id)
    }
}
