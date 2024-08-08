use std::sync::Arc;

use futures::lock::Mutex;
use hark::{
    connection::ConnectionPool,
    settings::{self},
    startup::{self, shutdown_signal},
    state::AppState,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let settings = settings::get_config("config.toml").expect("Failed to read config");

    let mut connection_pool = ConnectionPool::new();
    for connection in settings.connections.iter() {
        connection_pool.spawn(connection.0.clone(), connection.1.clone());
    }

    let addr = (settings.server.host.as_str(), settings.server.port);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to the tcp stream");

    let connection_pool = Arc::new(Mutex::new(connection_pool));

    let state = AppState {
        connection_pool: Arc::clone(&connection_pool),
    };

    let server = startup::run(listener, settings, state)
        .await
        .expect("Failed to bind the server");

    server.with_graceful_shutdown(shutdown_signal()).await?;

    tracing::info!("Stopping all connection tasks...");
    let mut lock = connection_pool.lock().await;
    lock.stop_all().await;
    lock.join_all().await;

    Ok(())
}
