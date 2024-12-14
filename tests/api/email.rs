use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct NewEmailUser {
    email: String,
    login: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailUser {
    pub login: String,
    pub email: String,
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

    assert_eq!(response.status().as_u16(), 200);

    EmailUser {
        login: body.login,
        email: body.email,
    }
}

pub async fn send_test_email(email_user: &EmailUser) {
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
