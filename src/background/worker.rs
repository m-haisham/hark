use std::sync::Arc;

use crate::{
    anchor::CallbackRequest,
    connection::types::{ConnectionEvent, ConnectionEventKind, ConnectionState},
    state::ArcAppState,
    task::TaskId,
};

use super::command::BackgroundCommand;

#[tracing::instrument(name = "Background Worker", skip_all, fields(id = %id))]
pub async fn background_worker(
    id: TaskId,
    state: ArcAppState,
    receiver: async_channel::Receiver<BackgroundCommand>,
) {
    let result = background_worker_inner(id, Arc::clone(&state), receiver).await;
    if let Err(err) = result {
        tracing::error!("Background worker {} failed: {:?}", id, err);
    }

    tracing::info!("Background worker {} stopped", id);
}

pub async fn background_worker_inner(
    task_id: TaskId,
    state: ArcAppState,
    receiver: async_channel::Receiver<BackgroundCommand>,
) -> eyre::Result<()> {
    loop {
        tracing::debug!("Background worker {task_id} waiting for command");

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
                ConnectionEventKind::Starting => {
                    tracing::debug!("Marking connection {} as starting", id);
                    let _ = state
                        .anchor
                        .send(CallbackRequest::ConnectionStarting { connection_id: id })
                        .await;
                }
                ConnectionEventKind::Started => {
                    tracing::debug!("Marking connection {} as started", id);

                    {
                        let mut lock = state.connection_pool.lock().await;
                        let Some(connection) = lock.get_connection_mut(&id) else {
                            tracing::warn!("Connection {} not found", id);
                            continue;
                        };

                        connection.state = ConnectionState::Running;
                        tracing::info!("Connection {} started", id);
                    }

                    let _ = state
                        .anchor
                        .send(CallbackRequest::ConnectionRunning { connection_id: id })
                        .await;
                }
                ConnectionEventKind::Stopped => {
                    tracing::debug!("Marking connection {} as stopped", id);

                    {
                        let mut lock = state.connection_pool.lock().await;
                        let Some(connection) = lock.get_connection_mut(&id) else {
                            tracing::warn!("Connection {} not found", id);
                            continue;
                        };

                        connection.state = ConnectionState::Stopped;
                        tracing::info!("Connection {} stopped", id);
                    }

                    let _ = state
                        .anchor
                        .send(CallbackRequest::ConnectionStopped { connection_id: id })
                        .await;
                }
                ConnectionEventKind::Failed(error) => {
                    tracing::debug!("Marking connection {} as failed", id);

                    {
                        let mut lock = state.connection_pool.lock().await;
                        let Some(connection) = lock.get_connection_mut(&id) else {
                            tracing::warn!("Connection {} not found", id);
                            continue;
                        };

                        connection.state = ConnectionState::Failed(error.clone());
                    }

                    let _ = state
                        .anchor
                        .send(CallbackRequest::ConnectionFailed {
                            connection_id: id,
                            error,
                        })
                        .await;
                }
                ConnectionEventKind::MessageReceived(message) => {
                    tracing::debug!("Received message for connection {}", id);

                    {
                        let lock = state.connection_pool.lock().await;
                        let Some(_) = lock.get_connection(&id) else {
                            tracing::warn!("Connection {} not found", id);
                            continue;
                        };
                    }

                    tracing::debug!("Sending message to callback URL for connection {}", id);

                    let _ = state
                        .anchor
                        .send(CallbackRequest::MessageReceived {
                            connection_id: id.clone(),
                            message,
                        })
                        .await;
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
