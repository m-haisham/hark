use crate::{
    connection::{
        task::ConnectionHandle,
        types::{Connection, ConnectionId},
    },
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
    let mut lock = state.connection_pool.lock().await;

    let NewConnection { name, connection } = data;
    let id = ConnectionId::try_from(name)
        .map_err(|e| ResponseError::BadRequest(anyhow!(e), e.to_string()))?;

    lock.spawn(id.clone(), connection);

    let connection = lock
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
            anyhow!("Connection not found"),
            id.to_string(),
        )),
    }
}
