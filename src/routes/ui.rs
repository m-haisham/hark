use std::{sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::HeaderName,
    response::{IntoResponse, Sse},
};
use tokio_stream::StreamExt;

use crate::{
    frontend::FrontendEvent,
    response::ResponseError,
    state::AppState,
    templates::{FrontendStyle, Index},
};

pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/", axum::routing::get(ui))
        .route("/sse", axum::routing::get(sse))
        .route("/dist/output.css", axum::routing::get(main_css))
}

pub async fn ui() -> Index {
    #[cfg(debug_assertions)]
    let style = FrontendStyle::Filesystem("dist/output.css".to_string());
    #[cfg(not(debug_assertions))]
    let style = FrontendStyle::Embedded(include_str!("../../dist/output.css"));

    crate::templates::Index { style }
}

#[tracing::instrument(skip_all)]
pub async fn sse(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stream = state.frontend.subscribe().filter_map(|result| {
        let event = match result {
            Ok(event) => event,
            Err(e) => {
                tracing::error!("Failed to receive event: {}", e);
                return None;
            }
        };

        match event.to_sse_event() {
            Ok(sse_event) => Some(Ok::<_, eyre::Report>(sse_event)),
            Err(e) => {
                tracing::error!("Failed to convert event to SSE event: {}", e);
                None
            }
        }
    });

    // Send the initial connection list to the frontend
    {
        let connection_pool = state.connection_pool.lock().await;
        match connection_pool.list_connection_info().await {
            Ok(connections) => {
                state.frontend.send(FrontendEvent::Connections(connections));
            }
            Err(e) => {
                tracing::error!("Failed to list connection info: {}", e);
            }
        }
    }

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

#[axum::debug_handler]
pub async fn main_css() -> Result<impl IntoResponse, ResponseError> {
    let path = std::path::Path::new("dist/output.css");
    let css =
        std::fs::read_to_string(path).map_err(|e| ResponseError::ServerError(eyre::eyre!(e)))?;
    Ok(([(HeaderName::from_static("content-type"), "text/css")], css))
}
