use std::sync::Arc;

use axum::{http::HeaderName, response::IntoResponse};

use crate::{response::ResponseError, state::AppState, templates::Index};

pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/", axum::routing::get(ui))
        .route("/dist/output.css", axum::routing::get(main_css))
}

pub async fn ui() -> Index {
    crate::templates::Index {}
}

#[axum::debug_handler]
pub async fn main_css() -> Result<impl IntoResponse, ResponseError> {
    let path = std::path::Path::new("dist/output.css");
    let css =
        std::fs::read_to_string(path).map_err(|e| ResponseError::ServerError(eyre::eyre!(e)))?;
    Ok(([(HeaderName::from_static("content-type"), "text/css")], css))
}
