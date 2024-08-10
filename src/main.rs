use std::sync::Arc;

use futures::lock::Mutex;
use hark::{
    background::BackgroundPool,
    connection::ConnectionPool,
    settings::{self},
    startup::{self, shutdown_signal},
    state::AppState,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let settings = settings::get_config("config.toml").expect("Failed to read config");

    let background_pool = BackgroundPool::new();

    let mut connection_pool = ConnectionPool::new();
    for (id, connection) in settings.connections.iter() {
        connection_pool.spawn(id.clone(), connection.clone(), background_pool.sender());
    }

    let addr = (settings.server.host.as_str(), settings.server.port);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to the tcp stream");

    let state = Arc::new(AppState {
        connection_pool: Mutex::new(connection_pool),
        background_pool: Mutex::new(background_pool),
        settings,
    });

    {
        let mut background_lock = state.background_pool.lock().await;
        background_lock.spawn(&state);
    }

    let server = startup::run(listener, Arc::clone(&state))
        .await
        .expect("Failed to bind the server");

    server.with_graceful_shutdown(shutdown_signal()).await?;

    {
        tracing::info!("Stopping all connection tasks...");
        let mut connection_lock = state.connection_pool.lock().await;
        connection_lock.stop_all().await;
        connection_lock.join_all().await;
    }

    {
        tracing::info!("Shutting down background workers...");
        let mut background_lock = state.background_pool.lock().await;
        background_lock.stop_all().await?;
        background_lock.join_all().await?;
    }

    Ok(())
}
