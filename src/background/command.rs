use crate::connection::types::{ConnectionEvent, ConnectionId};

#[derive(Debug)]
pub enum BackgroundCommand {
    ConnectionEvent(ConnectionEvent),
    SessionEvent(SessionEvent),
    CloseSession(ConnectionId),
    RestartSession(ConnectionId),
    Stop,
}

#[derive(Debug)]
pub enum SessionEvent {
    Started(ConnectionId),
    Exited(ConnectionId),
}
