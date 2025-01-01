use core::str;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use crate::email::{create_email_user, send_test_email};
use crate::helpers::{get_test_settings, spawn_app, spawn_app_with_settings, wait_until_running};
use crate::matchers::callback_type;
use crate::routes::connection::{create_connection, new_connection};

#[tokio::test]
async fn connection_should_send_message_to_callback() {
    // Arrange
    let email_user = create_email_user("callback").await;
    let mut connection = new_connection("test");
    connection["username"] = email_user.login.clone().into();

    let app = spawn_app().await;
    create_connection(&app, &connection).await;
    wait_until_running(&app, "test").await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("message_received"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock_server)
        .await;

    // Act
    send_test_email(&email_user).await;

    // Assert
    wait_until_callback_is_called(&app.mock_server, "message_received", 1).await;
}

pub async fn wait_until_callback_is_called(
    mock_server: &MockServer,
    kind: &str,
    expected_requests: usize,
) {
    let mut last_count = 0;

    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let requests: Vec<wiremock::Request> = mock_server
            .received_requests()
            .await
            .expect("Failed to get requests.");

        let message_received_requests = requests
            .iter()
            .flat_map(|r| str::from_utf8(&r.body).ok())
            .filter(|b| b.contains(kind))
            .collect::<Vec<_>>();

        if message_received_requests.len() >= expected_requests {
            return;
        }

        last_count = message_received_requests.len();
    }

    panic!("Callback '{kind}' was called {last_count} times, but expected {expected_requests}.");
}

#[tokio::test]
async fn lazy_session_should_restart_after_error_if_events_pending() {
    // Arrange
    let email_user = create_email_user("lazy-after-timeout").await;
    let mut connection = new_connection("test");
    connection["username"] = email_user.login.clone().into();

    let mut settings = get_test_settings();
    settings.lazy.max_fetch_count = 1.into(); // Limit fetches to 1, so we can test session restart
    let app = spawn_app_with_settings(settings).await;

    create_connection(&app, &connection).await;
    wait_until_running(&app, "test").await;

    let email_count = 3;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("message_received"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(email_count)
        .mount(&app.mock_server)
        .await;

    // Act
    for _ in 0..email_count {
        send_test_email(&email_user).await;
    }

    // Assert
    wait_until_callback_is_called(&app.mock_server, "message_received", email_count as usize).await;
}
