use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tokio::{net::TcpListener, signal};
use tower_http::trace::TraceLayer;

use crate::{
    routes::{self, connection, health_check},
    state::AppState,
    tracing::RequestSpan,
};

pub type Server = axum::serve::Serve<TcpListener, axum::Router, axum::Router>;

pub async fn run(listener: TcpListener, state: Arc<AppState>) -> Result<Server, std::io::Error> {
    let host = state.settings.server.host.clone();
    let port = state.settings.server.port;

    let app = Router::new()
        .route("/health-check", get(health_check))
        .route("/test-connection", post(connection::test_connection))
        .route("/connections", get(connection::list_connections))
        .route("/connections", post(connection::create_connection))
        .route("/connections/{id}", get(connection::get_connection))
        .route("/connections/{id}", put(connection::update_connection))
        .route("/connections/{id}", delete(connection::delete_connection))
        .merge(routes::ui::routes())
        .layer(TraceLayer::new_for_http().make_span_with(RequestSpan))
        .with_state(state);

    tracing::info!("Starting server at {host}:{port}");

    let server = axum::serve(listener, app);

    Ok(server)
}

pub async fn shutdown_signal(state: Arc<AppState>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down");
            state.frontend.shutdown();
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, shutting down");
            state.frontend.shutdown();
        },
    }
}
