use std::{collections::HashMap, sync::Arc};

use super::types::{Connection, ConnectionHandle, ConnectionId};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::instrument;

use crate::{
    background::command::BackgroundCommand,
    connection::{
        task::{run_connection_task, ConnectionTask},
        types::ConnectionInfo,
    },
    data::Data,
    settings::Settings,
};

#[derive(Debug)]
pub struct ConnectionPool {
    data: Arc<Data>,
    pool: HashMap<ConnectionId, ConnectionHandle>,
    handles: HashMap<ConnectionId, JoinHandle<()>>,
}

impl ConnectionPool {
    pub fn new(data: &Arc<Data>) -> Self {
        Self {
            data: Arc::clone(data),
            pool: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    #[instrument(name = "Spawn Connection Task", skip(self, connection))]
    pub fn spawn(
        &mut self,
        id: ConnectionId,
        connection: Connection,
        settings: &Settings,
        background: async_channel::Sender<BackgroundCommand>,
    ) {
        // Insert the connection into the data store
        self.data
            .connections
            .insert(id.clone(), Mutex::new(connection));

        let (sender, receiver) = tokio::sync::mpsc::channel(20);

        let task = ConnectionTask {
            id: id.clone(),
            data: Arc::clone(&self.data),
            receiver,
            background,
            idle_settings: settings.idle.clone(),
        };

        let handle = ConnectionHandle::new(id.clone(), Arc::clone(&self.data), sender);
        self.pool.insert(id.clone(), handle);

        let join_handle = tokio::spawn(run_connection_task(task));
        self.handles.insert(id.clone(), join_handle);

        tracing::debug!("Spawned connection task: {:?}", id);
    }

    pub fn list_connections(&self) -> impl Iterator<Item = (&ConnectionId, &ConnectionHandle)> {
        self.pool.iter()
    }

    pub async fn list_connection_info(&self) -> eyre::Result<Vec<ConnectionInfo>> {
        let mut connections = Vec::new();
        for (_, connection) in self.list_connections() {
            connections.push(connection.info().await?);
        }

        Ok(connections)
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
