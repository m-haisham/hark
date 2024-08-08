use futures::lock::Mutex;

use crate::{connection::ConnectionPool, settings::Settings};

#[derive(Debug)]
pub struct AppState {
    pub connection_pool: Mutex<ConnectionPool>,
    pub settings: Settings,
}
