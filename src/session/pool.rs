use std::{collections::HashMap, sync::Arc};

use eyre::{eyre, Context};

use crate::{
    background::command::BackgroundCommand,
    connection::types::ConnectionId,
    data::Data,
    session::lazy::{ImapLazySession, LazyCommand},
    settings::LazySettings,
};

#[derive(Debug)]
pub struct SessionPool {
    data: Arc<Data>,
    pool: HashMap<ConnectionId, ImapLazySession>,
    background_sender: async_channel::Sender<BackgroundCommand>,
    lazy_settings: LazySettings,
}

impl SessionPool {
    pub fn new(
        data: Arc<Data>,
        background_sender: async_channel::Sender<BackgroundCommand>,
        lazy_settings: LazySettings,
    ) -> Self {
        Self {
            data,
            pool: HashMap::new(),
            background_sender,
            lazy_settings,
        }
    }

    #[tracing::instrument(skip_all, fields(connection_id = %connection_id, command = ?command))]
    pub async fn send_command(
        &mut self,
        connection_id: ConnectionId,
        command: LazyCommand,
    ) -> eyre::Result<()> {
        if let Some(session) = self.pool.get(&connection_id) {
            tracing::info!(
                "Sending command to existing session handle: {}",
                connection_id
            );

            session
                .send(command)
                .await
                .map_err(|e| eyre!(e))
                .wrap_err("Failed to send command")?;
        } else {
            tracing::info!("Creating new session handle: {}", connection_id);

            let session: ImapLazySession = ImapLazySession::new(
                connection_id.clone(),
                Arc::clone(&self.data),
                self.background_sender.clone(),
                self.lazy_settings.clone(),
            );

            session
                .send(command)
                .await
                .map_err(|e| eyre!(e))
                .wrap_err("Failed to send command")?;

            self.pool.insert(connection_id, session);
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(connection_id = %connection_id))]
    pub async fn start_if_not_running(&self, connection_id: ConnectionId) -> eyre::Result<()> {
        let Some(session) = self.pool.get(&connection_id) else {
            tracing::error!("Tried to start non-existing session: {}", connection_id);
            return Ok(());
        };

        if session.is_running().await? {
            return Ok(());
        }

        session.start().await.wrap_err("Failed to start session")?;

        Ok(())
    }

    pub async fn stop_all(&mut self) {
        for (_, handle) in self.pool.iter_mut() {
            handle.stop().await.expect("Failed to stop session");
        }
    }

    pub async fn join_all(&mut self) {
        for (_, handle) in self.pool.iter_mut() {
            handle.wait_for_exit().await;
        }
    }
}
