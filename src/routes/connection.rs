use crate::{
    connection::types::{Connection, ConnectionHandle, ConnectionId},
    response::ResponseError,
    state::ArcAppState,
};
use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NewConnection {
    pub name: String,
    #[serde(flatten)]
    pub connection: Connection,
}

#[tracing::instrument(
    name = "Create a new connection",
    skip_all,
    fields(
        name = %data.name,
        host = %data.connection.host,
        port = %data.connection.port,
        username = %data.connection.username,
        mailbox = %data.connection.mailbox,
    ),
)]
pub async fn create_connection(
    State(state): State<ArcAppState>,
    Json(data): Json<NewConnection>,
) -> Result<Json<ConnectionHandle>, ResponseError> {
    let mut connection_lock = state.connection_pool.lock().await;
    let background_lock = state.background_pool.lock().await;

    let NewConnection { name, connection } = data;
    let id = ConnectionId::try_from(name)
        .map_err(|e| ResponseError::BadRequest(anyhow!(e), e.to_string()))?;

    connection_lock.spawn(id.clone(), connection, background_lock.sender());

    let connection = connection_lock
        .get_connection(&id)
        .cloned()
        .context("Failed to get connection from pool")?;

    Ok(Json(connection))
}

#[tracing::instrument(name = "List all connections", skip_all)]
pub async fn list_connections(State(state): State<ArcAppState>) -> Json<Vec<ConnectionHandle>> {
    let lock = state.connection_pool.lock().await;
    let connections = lock
        .list_connections()
        .map(|(_, connection)| connection.clone())
        .collect();

    Json(connections)
}

#[tracing::instrument(name = "Get a connection", skip_all, fields(id = %id))]
pub async fn get_connection(
    State(state): State<ArcAppState>,
    Path(id): Path<ConnectionId>,
) -> Result<Json<ConnectionHandle>, ResponseError> {
    let lock = state.connection_pool.lock().await;
    match lock.get_connection(&id) {
        Some(connection) => Ok(Json(connection.clone())),
        None => Err(ResponseError::NotFound(
            anyhow!("Connection not found: {id}"),
            id.to_string(),
        )),
    }
}

#[tracing::instrument(
    name = "Update a connection",
    skip_all,
    fields(
        id = %id,
        host = %data.host,
        port = %data.port,
        username = %data.username,
        mailbox = %data.mailbox,
    ),
)]
pub async fn update_connection(
    State(state): State<ArcAppState>,
    Path(id): Path<ConnectionId>,
    Json(data): Json<Connection>,
) -> Result<Json<ConnectionHandle>, ResponseError> {
    delete_connection_inner(&state, &id).await?;

    let mut connection_lock = state.connection_pool.lock().await;
    let background_lock = state.background_pool.lock().await;

    connection_lock.spawn(id.clone(), data, background_lock.sender());
    let connection = connection_lock
        .get_connection(&id)
        .cloned()
        .context("Failed to get connection from pool")?;

    Ok(Json(connection))
}

#[tracing::instrument(name = "Delete a connection", skip_all, fields(id = %id))]
pub async fn delete_connection(
    State(state): State<ArcAppState>,
    Path(id): Path<ConnectionId>,
) -> Result<Json<ConnectionHandle>, ResponseError> {
    Ok(Json(delete_connection_inner(&state, &id).await?))
}

async fn delete_connection_inner(
    state: &ArcAppState,
    id: &ConnectionId,
) -> Result<ConnectionHandle, ResponseError> {
    let mut lock = state.connection_pool.lock().await;

    let Some(mut connection) = lock.remove_connection(&id) else {
        return Err(ResponseError::NotFound(
            anyhow!("Connection not found: {id}"),
            "Connection not found".to_string(),
        ));
    };

    let join_handle = lock.remove_join(&id).await;
    drop(lock);

    connection.stop().await;

    if let Some(handle) = join_handle {
        if !handle.is_finished() {
            let _ = handle.await;
        }
    }

    Ok(connection)
}
