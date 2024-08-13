use serde::Serialize;

use crate::{connection::types::ConnectionId, imap::types::Message};

#[derive(Debug, Clone, Serialize)]
pub enum CallbackRequest {
    MessageReceived {
        connection_id: ConnectionId,
        message: Message,
    },
}
