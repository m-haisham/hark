use std::{sync::Arc, time::Duration};

use axum::{
    extract::State,
    response::{IntoResponse, Sse},
};
use tokio_stream::StreamExt;
use tower_http::services::{ServeDir, ServeFile};

use crate::{frontend::FrontendEvent, state::AppState};

pub fn routes() -> axum::Router<Arc<AppState>> {
    let serve_dir =
        ServeDir::new("dist/frontend").fallback(ServeFile::new("dist/frontend/index.html"));

    axum::Router::new()
        .route("/sse", axum::routing::get(sse))
        .fallback_service(serve_dir)
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
