use std::sync::Arc;

use futures::lock::Mutex;
use hark::{
    anchor::Anchor,
    background::BackgroundPool,
    connection::ConnectionPool,
    settings::{self, AnchorSettings},
    startup::{self, shutdown_signal},
    state::AppState,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let mut settings = settings::get_config("config.toml").expect("Failed to read config");
    let anchor = create_anchor(settings.anchor.clone()).await;

    // Merge the fetched connections with the existing ones
    if let Some(response) = anchor
        .fetch()
        .await
        .expect("Failed to fetch initialization data")
    {
        tracing::info!("Fetched response: {:?}", response);
        for (id, connection) in response.connections {
            settings.connections.insert(id, connection);
        }
    }

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
        anchor,
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

async fn create_anchor(anchor_settings: AnchorSettings) -> Anchor {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build reqwest client");

    // Copy the ping setting
    let ping = anchor_settings.ping;
    let anchor = Anchor::new(client.clone(), anchor_settings);

    if ping {
        anchor
            .ping()
            .await
            .expect("Failed to ping the callback URL");
    }

    anchor
}
