use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{routes::health_check, settings::Settings, tracing::RequestSpan};

pub type Server = axum::serve::Serve<axum::Router, axum::Router>;

pub async fn run(listener: TcpListener, settings: Settings) -> Result<Server, std::io::Error> {
    let (host, port) = (settings.server.host.clone(), settings.server.port);

    let app = Router::new()
        .route("/health-check", get(health_check))
        .layer(TraceLayer::new_for_http().make_span_with(RequestSpan));

    tracing::info!("Starting server at {host}:{port}");

    let server = axum::serve(listener, app);

    Ok(server)
}
