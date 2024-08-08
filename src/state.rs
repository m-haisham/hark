use std::sync::Arc;

use futures::lock::Mutex;

use crate::connection::ConnectionPool;

#[derive(Debug, Clone)]
pub struct AppState {
    pub connection_pool: Arc<Mutex<ConnectionPool>>,
}
