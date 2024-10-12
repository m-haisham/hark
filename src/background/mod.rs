pub mod command;
pub mod worker;

use crate::{state::ArcAppState, task::TaskId};
use eyre::Context;
use std::sync::Arc;
use tokio::task::JoinHandle;
use worker::background_worker;

pub struct BackgroundPool {
    task_id: TaskId,
    workers: Vec<JoinHandle<()>>,
    sender: async_channel::Sender<command::BackgroundCommand>,
    receiver: async_channel::Receiver<command::BackgroundCommand>,
}

impl BackgroundPool {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::bounded(255);

        Self {
            task_id: TaskId::new(0),
            workers: vec![],
            sender,
            receiver,
        }
    }

    pub fn spawn(&mut self, state: &ArcAppState) {
        self.task_id = self.task_id.next();

        let handle = tokio::spawn(background_worker(
            self.task_id,
            Arc::clone(state),
            self.receiver.clone(),
        ));

        self.workers.push(handle);
    }

    pub fn sender(&self) -> async_channel::Sender<command::BackgroundCommand> {
        self.sender.clone()
    }

    pub async fn stop_all(&mut self) -> eyre::Result<()> {
        tracing::debug!("Sending stop command to all background workers");

        for _ in 0..self.workers.len() {
            self.sender
                .send(command::BackgroundCommand::Stop)
                .await
                .map_err(|e| eyre::eyre!(e))
                .wrap_err("Failed to send stop command")?;
        }

        Ok(())
    }

    pub async fn join_all(&mut self) -> eyre::Result<()> {
        for worker in self.workers.drain(..) {
            worker
                .await
                .map_err(|e| eyre::eyre!(e))
                .wrap_err("Failed to join background worker")?;
        }

        Ok(())
    }
}
