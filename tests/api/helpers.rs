use std::future::IntoFuture;

use hark::{
    settings::{get_config, Settings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub settings: Settings,
    pub api_client: reqwest::Client,
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

    // During testing the working directory is the package directory.
    let settings = get_config("config.test.toml").expect("Failed to read configuration");

    let server = run(listener, settings.clone())
        .await
        .expect("Failed to bind the server");

    let _ = tokio::spawn(server.into_future());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        settings,
        api_client: client,
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
