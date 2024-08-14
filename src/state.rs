use std::sync::Arc;

use futures::lock::Mutex;

use crate::{
    anchor::Anchor, background::BackgroundPool, connection::ConnectionPool, settings::Settings,
};

pub type ArcAppState = Arc<AppState>;

#[derive(Debug)]
pub struct AppState {
    pub connection_pool: Mutex<ConnectionPool>,
    pub background_pool: Mutex<BackgroundPool>,
    pub anchor: Anchor,
    pub settings: Settings,
}
