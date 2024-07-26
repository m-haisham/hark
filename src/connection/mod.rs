use std::collections::HashMap;

use task::ConnectionHandle;
use types::Connection;

use crate::task::TaskId;

pub mod task;
pub mod types;

#[derive(Debug)]
pub struct ConnectionPool {
    pub current_id: TaskId,
    pub connections: HashMap<TaskId, ConnectionHandle>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            current_id: TaskId::new(0),
            connections: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, name: String, connection: Connection) -> TaskId {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);

        let id = TaskId::new(0);

        let task = task::ConnectionTask {
            id,
            key: name.clone(),
            connection,
            receiver,
        };

        let handle = ConnectionHandle { key: name, sender };
        self.connections.insert(id, handle);

        tokio::spawn(task::run_connection_task(task));

        id
    }
}
