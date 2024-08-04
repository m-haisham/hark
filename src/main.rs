use hark::{
    connection::ConnectionPool,
    settings::{self},
    startup,
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

    tracing::info!("Sleeping for 20 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    tracing::info!("Stopping all connections...");
    connection_pool.stop_all().await;
    connection_pool.join_all().await;

    std::process::exit(1);

    let addr = (settings.server.host.as_str(), settings.server.port);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to the tcp stream");

    let server = startup::run(listener, settings)
        .await
        .expect("Failed to bind the server");

    server.await
}
