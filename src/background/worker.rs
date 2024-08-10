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
    let result = background_worker_inner(task_id, Arc::clone(&state), receiver).await;
    if let Err(err) = result {
        tracing::error!("Background worker {} failed: {:?}", task_id, err);
    }

    tracing::info!("Background worker {} stopped", task_id);
}

pub async fn background_worker_inner(
    task_id: TaskId,
    state: ArcAppState,
    receiver: async_channel::Receiver<BackgroundCommand>,
) -> anyhow::Result<()> {
    loop {
        tracing::debug!("Background worker {{{task_id}}} waiting for command");

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
                    tracing::debug!("Marking connection {} as started", id);
                    let mut lock = state.connection_pool.lock().await;
                    let Some(connection) = lock.get_connection_mut(&id) else {
                        tracing::warn!("Connection {} not found", id);
                        continue;
                    };

                    connection.state = ConnectionState::Running;
                    tracing::info!("Connection {} started", id);
                }
                ConnectionEventKind::Stopped => {
                    tracing::debug!("Marking connection {} as stopped", id);
                    let mut lock = state.connection_pool.lock().await;
                    let Some(connection) = lock.get_connection_mut(&id) else {
                        tracing::warn!("Connection {} not found", id);
                        continue;
                    };

                    connection.state = ConnectionState::Stopped;
                    tracing::info!("Connection {} stopped", id);
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
