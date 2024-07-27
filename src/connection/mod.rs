use std::collections::HashMap;

use task::ConnectionHandle;
use types::{Connection, ConnectionId};

pub mod task;
pub mod types;

#[derive(Debug)]
pub struct ConnectionPool {
    pub connections: HashMap<ConnectionId, Connection>,
    pub handles: HashMap<ConnectionId, ConnectionHandle>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            handles: HashMap::new(),
        }
    }

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
        self.handles.insert(id, handle);

        tokio::spawn(task::run_connection_task(task));
    }
}
