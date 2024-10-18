use chrono::Utc;
use eyre::Context;
use task::{imap_connection_config, refresh_access_token};
use types::{Connection, ConnectionAuth, ConnectionId};

use crate::imap::{imap_connect_tcp, imap_connect_tls};

pub mod pool;
pub mod task;
pub mod types;

pub async fn imap_test_connect(id: ConnectionId, mut connection: Connection) -> eyre::Result<()> {
    if let ConnectionAuth::OAuth2(oauth2) = connection.auth {
        // Update the access token if it is about to expire, expired, or expires_at is not provided
        if (oauth2.expires_at.unwrap_or_else(Utc::now) - Utc::now()).num_seconds() < 60 {
            connection.auth = ConnectionAuth::OAuth2(refresh_access_token(&id, oauth2).await?);
        } else {
            connection.auth = ConnectionAuth::OAuth2(oauth2);
        }
    }

    let imap_connection = imap_connection_config(&connection).await?;

    if connection.tls {
        let mut session = imap_connect_tls(&imap_connection)
            .await
            .map_err(|e| eyre::eyre!(e))
            .wrap_err("Failed to connect to IMAP server")?;

        session.close().await?;
    } else {
        let mut session = imap_connect_tcp(&imap_connection)
            .await
            .map_err(|e| eyre::eyre!(e))
            .wrap_err("Failed to connect to IMAP server")?;

        session.close().await?;
    }

    Ok(())
}
