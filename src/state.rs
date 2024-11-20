use std::sync::Arc;

use futures::lock::Mutex;

use crate::{
    anchor::Anchor, background::BackgroundPool, connection::pool::ConnectionPool, data::Data,
    settings::Settings,
};

pub type ArcAppState = Arc<AppState>;

#[derive(Debug)]
pub struct AppState {
    pub data: Arc<Data>,
    pub connection_pool: Mutex<ConnectionPool>,
    pub background_pool: Mutex<BackgroundPool>,
    pub anchor: Anchor,
    pub settings: Settings,
}
