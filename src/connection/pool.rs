use std::{collections::HashMap, sync::Arc};

use super::types::{Connection, ConnectionHandle, ConnectionId};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::task::{run_connection_task, ConnectionTask},
    imap::lazy::ImapLazySession,
    settings::LazySettings,
};

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
        lazy_settings: &LazySettings,
        background: async_channel::Sender<BackgroundCommand>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::channel(20);

        let lazy = ImapLazySession::new(
            id.clone(),
            lazy_settings.timeout,
            lazy_settings.heartbeat,
            background.clone(),
        );

        let lazy = Arc::new(tokio::sync::Mutex::new(lazy));

        let task = ConnectionTask {
            id: id.clone(),
            connection: connection.clone(),
            receiver,
            background,
            lazy: Arc::clone(&lazy),
        };

        let handle = ConnectionHandle::new(id.clone(), connection, sender, lazy);
        self.pool.insert(id.clone(), handle);

        let join_handle = tokio::spawn(run_connection_task(task));
        self.handles.insert(id.clone(), join_handle);

        tracing::debug!("Spawned connection task: {:?}", id);
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
        for (_, handle) in self.pool.iter_mut() {
            handle.stop().await;
        }
    }

    pub async fn join_all(&mut self) {
        for (_, handle) in self.pool.iter_mut() {
            handle.wait_for_exit().await;
        }

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
