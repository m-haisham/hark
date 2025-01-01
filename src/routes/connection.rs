use crate::{
    connection::{
        imap_test_connect,
        types::{Connection, ConnectionId, ConnectionInfo},
    },
    response::ResponseError,
    state::ArcAppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use eyre::eyre;
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
) -> Result<Json<ConnectionInfo>, ResponseError> {
    let NewConnection { name, connection } = data;
    let id = ConnectionId::try_from(name)
        .map_err(|e| ResponseError::BadRequest(eyre!(e), e.to_string()))?;

    let background_lock = state.background_pool.lock().await;
    let mut connection_lock = state.connection_pool.lock().await;

    connection_lock.spawn(
        id.clone(),
        connection,
        &state.settings,
        background_lock.sender(),
    );

    let connection = connection_lock
        .get_connection(&id)
        .ok_or_else(|| eyre::eyre!("Failed to get connection from pool"))?
        .info()
        .await?;

    Ok(Json(connection))
}

pub async fn test_connection(Json(data): Json<NewConnection>) -> Result<Json<()>, ResponseError> {
    let NewConnection { name, connection } = data;
    let id = ConnectionId::try_from(name)
        .map_err(|e| ResponseError::BadRequest(eyre!(e), e.to_string()))?;

    imap_test_connect(id, connection).await.map_err(|e| {
        ResponseError::BadRequest(e, "Failed to connect to IMAP server".to_string())
    })?;

    Ok(Json(()))
}

#[tracing::instrument(name = "List all connections", skip_all)]
pub async fn list_connections(
    State(state): State<ArcAppState>,
) -> Result<Json<Vec<ConnectionInfo>>, ResponseError> {
    let lock = state.connection_pool.lock().await;

    let mut connections = Vec::new();
    for (_, connection) in lock.list_connections() {
        connections.push(connection.info().await?);
    }

    Ok(Json(connections))
}

#[tracing::instrument(name = "Get a connection", skip_all, fields(id = %id))]
pub async fn get_connection(
    State(state): State<ArcAppState>,
    Path(id): Path<ConnectionId>,
) -> Result<Json<ConnectionInfo>, ResponseError> {
    let lock = state.connection_pool.lock().await;
    match lock.get_connection(&id) {
        Some(connection) => Ok(Json(connection.info().await?)),
        None => Err(ResponseError::NotFound(
            eyre!("Connection not found: {id}"),
            "Connection not found".to_string(),
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
) -> Result<Json<ConnectionInfo>, ResponseError> {
    delete_connection_inner(&state, &id).await?;

    let mut connection_lock = state.connection_pool.lock().await;
    let background_lock = state.background_pool.lock().await;

    connection_lock.spawn(id.clone(), data, &state.settings, background_lock.sender());

    let connection = connection_lock
        .get_connection(&id)
        .ok_or_else(|| eyre::eyre!("Failed to get connection from pool"))?
        .info()
        .await?;

    Ok(Json(connection))
}

#[tracing::instrument(name = "Delete a connection", skip_all, fields(id = %id))]
pub async fn delete_connection(
    State(state): State<ArcAppState>,
    Path(id): Path<ConnectionId>,
) -> Result<Json<ConnectionInfo>, ResponseError> {
    Ok(Json(delete_connection_inner(&state, &id).await?))
}

async fn delete_connection_inner(
    state: &ArcAppState,
    id: &ConnectionId,
) -> Result<ConnectionInfo, ResponseError> {
    let mut lock = state.connection_pool.lock().await;

    let info_before_deletion = {
        let Some(connection) = lock.get_connection(&id) else {
            return Err(ResponseError::NotFound(
                eyre!("Connection not found: {id}"),
                "Connection not found".to_string(),
            ));
        };

        connection.stop().await;
        connection.info().await?
    };

    let join_handle = lock.remove_join(&id).await;
    drop(lock);

    if let Some(handle) = join_handle {
        if !handle.is_finished() {
            let _ = handle.await;
        }
    }

    let mut lock = state.connection_pool.lock().await;

    let info = if let Some(connection) = lock.remove_connection(&id) {
        if let Ok(info) = connection.info().await {
            info
        } else {
            tracing::warn!("Failed to get connection info after deletion");
            info_before_deletion
        }
    } else {
        return Err(ResponseError::ServerError(eyre!(
            "Failed to delete connection"
        )));
    };

    Ok(info)
}
