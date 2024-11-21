use crate::connection::types::{ConnectionEvent, ConnectionId};

#[derive(Debug)]
pub enum BackgroundCommand {
    ConnectionEvent(ConnectionEvent),
    RestartSession(ConnectionId),
    Stop,
}
