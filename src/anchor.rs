use std::collections::HashMap;

use anyhow::Context;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    connection::types::{Connection, ConnectionId},
    imap::types::Message,
    settings::AnchorSettings,
};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum CallbackRequest {
    MessageReceived {
        connection_id: ConnectionId,
        message: Message,
    },
    Ping,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FetchResponse {
    #[serde(default)]
    pub connections: HashMap<ConnectionId, Connection>,
}

#[derive(Debug)]
pub struct Anchor {
    client: reqwest::Client,
    settings: AnchorSettings,
}

impl Anchor {
    pub fn new(client: reqwest::Client, settings: AnchorSettings) -> Self {
        Self { client, settings }
    }

    pub async fn new_with_ping(
        client: reqwest::Client,
        settings: AnchorSettings,
    ) -> anyhow::Result<Self> {
        let anchor = Self { client, settings };
        anchor.ping().await?;
        Ok(anchor)
    }

    pub async fn send(&self, request: CallbackRequest) -> anyhow::Result<()> {
        let id = match &request {
            CallbackRequest::MessageReceived { connection_id, .. } => Some(connection_id.clone()),
            CallbackRequest::Ping => None,
        };

        let callback_url = self.settings.callback_url.as_str();
        let result = self.client.post(callback_url).json(&request).send().await;

        match result {
            Ok(response) if response.status() != StatusCode::OK => {
                tracing::warn!(
                    "The server replied with unexpected status code (connection={id:?}): {}",
                    response.status(),
                );

                Err(anyhow::anyhow!(
                    "The server replied with unexpected status code (connection={id:?}): {}",
                    response.status(),
                ))
            }
            Ok(_) => {
                tracing::info!("Message sent to callback URL (connection={id:?})");
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send message to callback URL (connection={id:?}): {e:?}",
                );

                Err(e).context("Failed to send message to callback URL")
            }
        }
    }

    pub async fn fetch(&self) -> anyhow::Result<Option<FetchResponse>> {
        let Some(fetch_url) = &self.settings.fetch_url else {
            return Ok(None);
        };

        let response = self
            .client
            .get(fetch_url.as_str())
            .send()
            .await
            .context("Failed to fetch connections")?;

        if response.status() != StatusCode::OK {
            tracing::warn!(
                "The server replied with unexpected status code: {}",
                response.status(),
            );

            return Err(anyhow::anyhow!(
                "The server replied with unexpected status code: {}",
                response.status(),
            ));
        }

        let response = response
            .json::<FetchResponse>()
            .await
            .context("Failed to parse response")?;

        Ok(Some(response))
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        tracing::info!("Pinging the anchor...");
        self.send(CallbackRequest::Ping).await?;
        Ok(())
    }
}
