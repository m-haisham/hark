use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use crate::helpers::{spawn_app, wait_until_running};
use crate::matchers::callback_type;
use crate::routes::connection::{create_connection, new_connection};

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
    wait_until_callback_is_called(&app.mock_server).await;
}

async fn send_test_email() {
    use lettre::{
        transport::smtp::{authentication::Credentials, client::Tls},
        AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    };

    let email = lettre::Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to("Test <username@example.com>".parse().unwrap())
        .subject("Happy new year")
        .header(lettre::message::header::ContentType::TEXT_PLAIN)
        .body(String::from("Be happy!"))
        .expect("Failed to create email.");

    let creds = Credentials::new("username".to_owned(), "password".to_owned());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
        .expect("Failed to create mailer.")
        .credentials(creds)
        .tls(Tls::None)
        .port(3025)
        .build();

    mailer.send(email).await.expect("Failed to send email.");
}

async fn wait_until_callback_is_called(mock_server: &MockServer) {
    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let requests = mock_server
            .received_requests()
            .await
            .expect("Failed to get requests.");

        if requests.len() > 2 {
            return;
        }
    }

    panic!("Callback was not called.");
}
