use std::sync::Arc;

use crate::{
    connection::types::{ConnectionEvent, ConnectionEventKind, ConnectionState},
    state::ArcAppState,
    task::TaskId,
};

use super::command::BackgroundCommand;

#[tracing::instrument(name = "Background Worker", skip_all, fields(task_id = %task_id))]
pub async fn background_worker(
    task_id: TaskId,
    state: ArcAppState,
    receiver: async_channel::Receiver<BackgroundCommand>,
) {
    let result = background_worker_inner(Arc::clone(&state), receiver).await;
    if let Err(err) = result {
        tracing::error!("Background worker {} failed: {:?}", task_id, err);
    }
}

pub async fn background_worker_inner(
    state: ArcAppState,
    receiver: async_channel::Receiver<BackgroundCommand>,
) -> anyhow::Result<()> {
    loop {
        let command_result = receiver.recv().await;
        let command = match command_result {
            Ok(command) => command,
            Err(_) => {
                tracing::info!("Background worker stopped due to channel closed");
                break;
            }
        };

        match command {
            BackgroundCommand::ConnectionEvent(ConnectionEvent { id, event }) => match event {
                ConnectionEventKind::Started => {
                    tracing::info!("Connection {} started", id);
                    let mut lock = state.connection_pool.lock().await;
                    let Some(connection) = lock.get_connection_mut(&id) else {
                        tracing::warn!("Connection {} not found", id);
                        continue;
                    };

                    connection.state = ConnectionState::Running;
                }
                ConnectionEventKind::Stopped => {
                    tracing::info!("Connection {} stopped", id);
                    let mut lock = state.connection_pool.lock().await;
                    let Some(connection) = lock.get_connection_mut(&id) else {
                        tracing::warn!("Connection {} not found", id);
                        continue;
                    };

                    connection.state = ConnectionState::Stopped;
                }
            },
            BackgroundCommand::Stop => {
                tracing::info!("Background worker received stop command");
                break;
            }
        }
    }

    Ok(())
}
