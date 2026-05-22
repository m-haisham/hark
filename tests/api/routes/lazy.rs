use wiremock::{
    matchers::{method, path},
    Mock,
};

use crate::{
    email::{create_email_user, send_test_email},
    helpers::{get_test_settings, spawn_app, spawn_app_with_settings, wait_until_running},
    matchers::callback_type,
    routes::{
        connection::{create_connection, delete_connection, new_connection},
        receive::wait_until_callback_is_called,
    },
};

#[tokio::test]
pub async fn lazy_session_should_not_start_when_connection_is_created() {
    // Arrange
    let app = spawn_app().await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("message_received"))
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
