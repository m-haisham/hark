use hark::{
    settings::{self},
    startup,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let settings = settings::get_config("config.toml").expect("Failed to read config");

    let listener =
        tokio::net::TcpListener::bind((settings.server.host.as_str(), settings.server.port))
            .await
            .expect("Failed to bind to the tcp stream");

    let server = startup::run(listener, settings)
        .await
        .expect("Failed to bind the server");

    server.await
}
