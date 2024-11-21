use std::{collections::HashMap, sync::Arc};

use eyre::{eyre, Context};

use crate::{
    background::command::BackgroundCommand,
    connection::types::ConnectionId,
    data::Data,
    imap::lazy::{ImapLazySession, LazyCommand},
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

    pub async fn send_command(
        &mut self,
        connection_id: ConnectionId,
        command: LazyCommand,
    ) -> eyre::Result<()> {
        if let Some(session) = self.pool.get(&connection_id) {
            session
                .send(command)
                .await
                .map_err(|e| eyre!(e))
                .wrap_err("Failed to send command")?;
        } else {
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
