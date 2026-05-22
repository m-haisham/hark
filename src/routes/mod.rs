pub mod connection;
pub mod ui;

use axum::{http::StatusCode, response::IntoResponse};

pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
