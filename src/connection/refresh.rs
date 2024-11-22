use chrono::Utc;
use eyre::{eyre, WrapErr};
use oauth2::TokenResponse;
use std::time::Duration;

use crate::data::Data;

use super::types::{Connection, ConnectionAuth, ConnectionId, OAuth2};

pub async fn get_connection_from_store(
    data: &Data,
    connection_id: &ConnectionId,
) -> eyre::Result<Connection> {
    let Some(connection) = data.connections.get(connection_id) else {
        return Err(eyre!("Connection not found"));
    };

    let lock = connection.lock().await;

    // Using static access to make sure the connection is cloned
    Ok(Connection::clone(&lock))
}

pub async fn is_connection_auth_refresh_needed(connection: &Connection) -> bool {
    if let ConnectionAuth::OAuth2(oauth2) = &connection.auth {
        // Update the access token if it is about to expire, expired, or expires_at is not provided
        if (oauth2.expires_at.unwrap_or_else(Utc::now) - Utc::now()).num_seconds() < 60 {
            return true;
        }
    }

    false
}

pub async fn refresh_connection_auth(
    data: &Data,
    connection_id: &ConnectionId,
) -> eyre::Result<Connection> {
    let Some(connection) = data.connections.get(connection_id) else {
        return Err(eyre!("Connection not found"));
    };

    let mut connection = connection.lock().await;

    if let ConnectionAuth::OAuth2(oauth2) = &mut connection.auth {
        *oauth2 = refresh_access_token(connection_id, oauth2).await?;
    }

    Ok(Connection::clone(&connection))
}

#[tracing::instrument(
    name = "Refresh access token",
    skip_all,
    fields(connection_id = %connection_id)
)]
pub async fn refresh_access_token(
    connection_id: &ConnectionId,
    oauth2: &OAuth2,
) -> eyre::Result<OAuth2> {
    let OAuth2 {
        refresh_token,
        config,
        ..
    } = oauth2;

    tracing::info!("Refreshing access token for connection");

    let client: oauth2::basic::BasicClient = oauth2::Client::new(
        config.client_id.clone(),
        Some(config.client_secret.clone()),
        config.auth_uri.clone(),
        Some(config.token_uri.clone()),
    );

    let request = client.exchange_refresh_token(&refresh_token);

    tracing::debug!("Requesting new access token");

    let response = request
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to refresh access token")?;

    let expires_in = response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));

    let expires_at = chrono::Utc::now() + chrono::Duration::from_std(expires_in)?
        - chrono::Duration::seconds(60); // subtract 60 seconds to be safe

    // Use the refresh token from the response if provided, otherwise use the existing one
    // This is to handle the case where the refresh token is rotated
    let refresh_token = response
        .refresh_token()
        .cloned()
        .unwrap_or_else(|| refresh_token.clone());

    let oauth2 = OAuth2 {
        access_token: response.access_token().clone(),
        expires_at: Some(expires_at),
        refresh_token,
        config: config.clone(),
    };

    Ok(oauth2)
}
