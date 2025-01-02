use wiremock::{
    matchers::{method, path},
    Mock,
};

use crate::{
    email::{create_email_user, send_test_email},
    helpers::{spawn_app, wait_until_running},
    matchers::callback_type,
    routes::{
        connection::{create_connection, new_connection},
        receive::wait_until_callback_is_called,
    },
};

use super::connection::delete_connection;

#[tokio::test]
pub async fn lazy_session_should_not_start_when_connection_is_created() {
    // Arrange
    let app = spawn_app().await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("session_started"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.mock_server)
        .await;

    // Act
    let response = app
        .api_client
        .post(&format!("{}/connections", app.address))
        .json(&new_connection("test"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
pub async fn lazy_session_should_start_when_message_is_received() {
    // Arrange
    let email_user = create_email_user("session-start").await;
    let mut connection = new_connection("test");
    connection["username"] = email_user.login.clone().into();

    let app = spawn_app().await;
    create_connection(&app, &connection).await;
    wait_until_running(&app, "test").await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("session_started"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock_server)
        .await;

    // Act
    send_test_email(&email_user).await;

    // Assert
    wait_until_callback_is_called(&app.mock_server, "session_started", 1).await;
}

#[tokio::test]
pub async fn lazy_session_should_close_after_connection_is_removed() {
    // Arrange
    let email_user = create_email_user("lazy-close").await;
    let mut connection = new_connection("test");
    connection["username"] = email_user.login.clone().into();

    let app = spawn_app().await;
    create_connection(&app, &connection).await;
    wait_until_running(&app, "test").await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("session_closed"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock_server)
        .await;

    // Act
    send_test_email(&email_user).await;
    wait_until_callback_is_called(&app.mock_server, "session_started", 1).await;
    delete_connection(&app, "test").await;

    // Assert
    wait_until_callback_is_called(&app.mock_server, "session_closed", 1).await;
}
