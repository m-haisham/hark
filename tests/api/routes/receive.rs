use lettre::AsyncTransport;
use wiremock::matchers::{method, path};
use wiremock::Mock;

use crate::helpers::{spawn_app, TestApp};
use crate::matchers::callback_type;
use crate::routes::connection::{create_connection, new_connection};

use super::connection::get_connection;

async fn send_test_email() {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, Tokio1Executor};

    let email = lettre::Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to("Hei <hei@domain.tld>".parse().unwrap())
        .subject("Happy new year")
        .header(lettre::message::header::ContentType::TEXT_PLAIN)
        .body(String::from("Be happy!"))
        .unwrap();

    let creds = Credentials::new("username".to_owned(), "password".to_owned());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
        .unwrap()
        .credentials(creds)
        .port(3143)
        .build();

    mailer.send(email).await.unwrap();
}

#[tokio::test]
async fn connection_should_send_message_to_callback() {
    // Arrange
    let app = spawn_app().await;
    create_connection(&app, new_connection("test")).await;
    wait_until_running(&app, "test").await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("message_received"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock_server)
        .await;

    // Act
    send_test_email().await;

    // Assert
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

async fn wait_until_running(app: &TestApp, id: &str) {
    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let connection = get_connection(&app, id).await;
        match connection["status"].as_str() {
            Some("running") => return,
            Some("stopped") => panic!("Connection stopped running."),
            Some("failed") => panic!("Connection failed to start."),
            _ => {}
        }
    }

    panic!("Connection did not start running. Make sure the IMAP server is running and the connection settings are correct.");
}
