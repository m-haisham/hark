use crate::connection::types::{ConnectionEvent, ConnectionId};

#[derive(Debug)]
pub enum BackgroundCommand {
    ConnectionEvent(ConnectionEvent),
    SessionEvent(SessionEvent),
    RestartSession(ConnectionId),
    Stop,
}

#[derive(Debug)]
pub enum SessionEvent {
    Started(ConnectionId),
    Exited(ConnectionId),
}
