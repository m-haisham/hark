pub mod connection;

use axum::{http::StatusCode, response::IntoResponse};

pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
