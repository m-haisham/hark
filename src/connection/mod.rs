use std::collections::HashMap;

use task::ConnectionHandle;
use tokio::task::JoinSet;
use types::Connection;

use crate::task::TaskId;

pub mod task;
pub mod types;

#[derive(Debug)]
pub struct ConnectionPool {
    pub current_id: TaskId,
    pub tasks: JoinSet<anyhow::Result<()>>,
    pub connections: HashMap<TaskId, ConnectionHandle>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            current_id: TaskId::new(0),
            tasks: JoinSet::new(),
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

    pub async fn join_wait(&mut self) {
        while let Some(res) = self.tasks.join_next().await {
            let out = res.unwrap();

            match out {
                Ok(_) => tracing::info!("Listening task completed successfully"),
                Err(e) => tracing::error!("Listening task failed: {:?}", e),
            }
        }
    }
}
