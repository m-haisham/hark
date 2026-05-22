use axum::response::sse::Event;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    connection::types::{ConnectionId, ConnectionInfo},
    imap::types::Message,
};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FrontendEvent {
    Connections(Vec<ConnectionInfo>),
    Message {
        connection_id: ConnectionId,
        message: Message,
    },
}

impl FrontendEvent {
    pub fn to_sse_event(&self) -> eyre::Result<Event> {
        let event = match self {
            FrontendEvent::Connections(_) => "connections",
            FrontendEvent::Message { .. } => "message",
        };

        let data = serde_json::to_string(self)
            .map_err(|e| eyre::eyre!("Failed to serialize event data: {}", e))?;

        Ok(Event::default().event(event).data(data))
    }
}

#[derive(Debug)]
pub struct FrontendBroadcaster {
    sender: broadcast::Sender<FrontendEvent>,
}

impl FrontendBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub fn subscribe(&self) -> BroadcastStream<FrontendEvent> {
        let receiver = self.sender.subscribe();
        BroadcastStream::new(receiver)
    }

    pub fn send(&self, event: FrontendEvent) {
        // The receiver will be dropped if there are no subscribers, so we can ignore the error.
        let _ = self.sender.send(event);
    }
}
