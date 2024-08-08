use std::sync::Arc;

use futures::lock::Mutex;

use crate::{connection::ConnectionPool, settings::Settings};

pub type ArcAppState = Arc<AppState>;

#[derive(Debug)]
pub struct AppState {
    pub connection_pool: Mutex<ConnectionPool>,
    pub settings: Settings,
}
