use core::str;

use serde::{Deserialize, Serialize};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use crate::helpers::{get_test_settings, spawn_app, spawn_app_with_settings, wait_until_running};
use crate::matchers::callback_type;
use crate::routes::connection::{create_connection, new_connection};

#[derive(Debug, Serialize)]
pub struct NewEmailUser {
    email: String,
    login: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailUser {
    login: String,
    email: String,
}

#[tracing::instrument]
pub async fn create_email_user(name: &str) -> EmailUser {
    let client = reqwest::Client::new();
    let url = "http://localhost:8080/api/user";

    let response = client.get(url).send().await.expect("Failed to get users.");
    assert_eq!(response.status().as_u16(), 200);

    let users = response
        .json::<Vec<EmailUser>>()
        .await
        .expect("Failed to parse users.");

    // Check if the user already exists
    let existing = users.into_iter().find(|u| u.login == name);
    if let Some(user) = existing {
        return user;
    }

    let body = NewEmailUser {
        email: format!("{}@example.com", name),
        login: name.to_string(),
        password: "password".to_string(),
    };

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .expect("Failed to create user.");

    assert_eq!(response.status().as_u16(), 201);

    EmailUser {
        login: body.login,
        email: body.email,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connection_should_send_message_to_callback() {
    // Arrange
    let email_user = create_email_user("callback").await;
    let mut connection = new_connection("test");
    connection["username"] = email_user.login.clone().into();

    let app = spawn_app().await;
    create_connection(&app, connection).await;
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
    wait_until_callback_is_called(&app.mock_server, 1).await;
}

async fn send_test_email(email_user: &EmailUser) {
    use lettre::{
        transport::smtp::{authentication::Credentials, client::Tls},
        AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    };

    let to = format!("{} <{}>", email_user.login, email_user.email);
    let to = to
        .parse()
        .expect(&format!("Failed to parse 'to' email: {}", to));

    let email = lettre::Message::builder()
        .from(
            "NoBody <nobody@domain.tld>"
                .parse()
                .expect("Failed to parse 'from' email"),
        )
        .reply_to(
            "Yuin <yuin@domain.tld>"
                .parse()
                .expect("Failed to parse 'reply-to' email"),
        )
        .to(to)
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

async fn wait_until_callback_is_called(mock_server: &MockServer, expected_requests: usize) {
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
            .filter(|b| b.contains("message_received"))
            .collect::<Vec<_>>();

        if message_received_requests.len() >= expected_requests {
            return;
        }

        last_count = message_received_requests.len();
    }

    panic!(
        "Callback was called {} times, but expected {}.",
        last_count, expected_requests
    );
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

    create_connection(&app, connection).await;
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
    wait_until_callback_is_called(&app.mock_server, email_count as usize).await;
}
