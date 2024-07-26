use std::collections::HashMap;

use hark::{
    connection::{
        task::{run_connection_task, ConnectionHandle, ConnectionTask},
        ConnectionPool,
    },
    settings::{self},
    task::TaskId,
};
use tokio::{sync::mpsc, task::JoinSet};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let settings = settings::get_config(".").expect("Failed to read config");

    let mut connection_pool = ConnectionPool::new();
    for (name, connection) in settings.connections {
        connection_pool.spawn(name, connection);
    }
    connection_pool.join_wait().await;

    Ok(())
}
