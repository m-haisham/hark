use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::connection::types::{Connection, ConnectionId};

#[derive(Debug)]
pub struct Data {
    pub connections: DashMap<ConnectionId, Mutex<Connection>>,
}

impl Data {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }
}
