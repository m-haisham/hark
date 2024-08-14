use std::{future::IntoFuture, sync::Arc};

use futures::lock::Mutex;
use hark::{
    anchor::Anchor,
    background::BackgroundPool,
    connection::ConnectionPool,
    settings::get_config,
    startup::run,
    state::AppState,
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;
use url::Url;
use wiremock::MockServer;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("debug".to_string(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("info".to_string(), std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub state: Arc<AppState>,
    pub api_client: reqwest::Client,
    pub mock_server: MockServer,
}

#[inline]
pub async fn spawn_app() -> TestApp {
    spawn_app_with_settings().await
}

pub async fn spawn_app_with_settings() -> TestApp {
    Lazy::force(&TRACING);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();

    let mock_server = MockServer::start().await;

    // During testing the working directory is the package directory.
    let mut settings = get_config("config.test.toml").expect("Failed to read configuration");
    settings.anchor.callback_url = Url::parse(&mock_server.uri())
        .expect("Failed to parse mock server URL")
        .join("/callback")
        .expect("Failed to join callback path");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let anchor = Anchor::new(client.clone(), settings.anchor.clone());

    let state = Arc::new(AppState {
        connection_pool: Mutex::new(ConnectionPool::new()),
        background_pool: Mutex::new(BackgroundPool::new()),
        anchor,
        settings: settings.clone(),
    });

    {
        let mut background_lock = state.background_pool.lock().await;
        background_lock.spawn(&state);
    }

    let server = run(listener, Arc::clone(&state))
        .await
        .expect("Failed to bind the server");

    let _ = tokio::spawn(server.into_future());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        state,
        api_client: client,
        mock_server,
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
