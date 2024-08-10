use crate::connection::types::ConnectionEvent;

#[derive(Debug)]
pub enum BackgroundCommand {
    ConnectionEvent(ConnectionEvent),
    Stop,
}
