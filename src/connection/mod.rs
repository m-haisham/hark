use std::collections::HashMap;

use task::ConnectionHandle;
use tokio::task::JoinHandle;
use tracing::instrument;
use types::{Connection, ConnectionCommand, ConnectionId};

pub mod task;
pub mod types;

#[derive(Debug)]
pub struct ConnectionPool {
    pub connections: HashMap<ConnectionId, Connection>,
    pub handles: HashMap<ConnectionId, ConnectionHandle>,
    pub join_handles: HashMap<ConnectionId, JoinHandle<()>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            handles: HashMap::new(),
            join_handles: HashMap::new(),
        }
    }

    #[instrument(name = "Spawn Connection Task", skip(self, connection))]
    pub fn spawn(&mut self, id: ConnectionId, connection: Connection) {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);

        let task = task::ConnectionTask {
            id: id.clone(),
            connection,
            receiver,
        };

        let handle = ConnectionHandle {
            id: id.clone(),
            sender,
        };
        self.handles.insert(id.clone(), handle);

        let join_handle = tokio::spawn(task::run_connection_task(task));
        self.join_handles.insert(id, join_handle);
    }

    pub async fn stop_all(&mut self) {
        for (_, handle) in self.handles.iter() {
            let _ = handle.sender.send(ConnectionCommand::Stop).await;
        }
    }

    pub async fn join_all(&mut self) {
        let mut handles = Vec::new();
        for (_, handle) in self.join_handles.drain() {
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }
}
