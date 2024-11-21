use std::{future::IntoFuture, sync::Arc};

use futures::lock::Mutex;
use hark::{
    anchor::Anchor,
    background::BackgroundPool,
    connection::pool::ConnectionPool,
    data::Data,
    session::pool::SessionPool,
    settings::{get_config, Settings},
    startup::run,
    state::AppState,
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;
use url::Url;
use wiremock::MockServer;

use crate::routes::connection::get_connection;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    if let Ok(level) = std::env::var("TEST_LOG") {
        let subscriber = get_subscriber(level, std::io::stdout);
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

pub fn get_test_settings() -> hark::settings::Settings {
    get_config("config.test.toml").expect("Failed to read configuration")
}

#[inline]
pub async fn spawn_app() -> TestApp {
    spawn_app_with_settings(get_test_settings()).await
}

pub async fn spawn_app_with_settings(mut settings: Settings) -> TestApp {
    Lazy::force(&TRACING);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();

    let mock_server = MockServer::start().await;

    // During testing the working directory is the package directory.
    settings.anchor.callback_url = Url::parse(&mock_server.uri())
        .expect("Failed to parse mock server URL")
        .join("/callback")
        .expect("Failed to join callback path");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let anchor = Anchor::new(client.clone(), settings.anchor.clone());

    let data = Arc::new(Data::new());
    let connection_pool = ConnectionPool::new(&data);

    let background_pool = BackgroundPool::new();

    let session_pool = SessionPool::new(
        Arc::clone(&data),
        background_pool.sender(),
        settings.lazy.clone(),
    );

    let state = Arc::new(AppState {
        data,
        connection_pool: Mutex::new(connection_pool),
        background_pool: Mutex::new(background_pool),
        session_pool: Mutex::new(session_pool),
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

pub async fn wait_until_running(app: &TestApp, id: &str) {
    let mut last_state = None;

    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let connection = get_connection(&app, id).await;

        tracing::debug!("Connection state: {:?}", connection);

        let state = connection["state"]["type"].as_str();
        match state {
            Some("running") => return,
            Some("stopped") => panic!("Connection stopped running."),
            Some("failed") => panic!("Connection failed to start."),
            _ => {}
        }

        last_state = state.map(|v| v.to_string());
    }

    panic!("Connection did not start running. Make sure the IMAP server is running and the connection settings are correct. Last state: {last_state:?}");
}

/// Wait until the connection reaches the specified state.
///
/// To check that a connection has reached the running state, use [`wait_until_running`] instead.
pub async fn wait_until_state(app: &TestApp, id: &str, state: &str) {
    let mut last_state = None;

    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let connection = get_connection(&app, id).await;

        tracing::debug!("Connection state: {:?}", connection);

        let current_state = connection["state"]["type"].as_str();
        if current_state == Some(state) {
            return;
        }

        last_state = current_state.map(|v| v.to_string());
    }

    panic!("Connection did not reach the state {state}. Last state: {last_state:?}");
}
