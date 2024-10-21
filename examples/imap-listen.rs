use std::str::FromStr;

use chrono::Duration;
use hark::{
    connection::types::ImapFlavour,
    imap::{
        imap_connect_tls, imap_idle, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig,
    },
};
use oauth2::AccessToken;
use secrecy::SecretString;
use tracing_subscriber::EnvFilter;

enum Auth {
    Password,
    OAuth2,
}

impl FromStr for Auth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "password" => Ok(Self::Password),
            "oauth2" => Ok(Self::OAuth2),
            _ => Err(format!("invalid auth type: {}", s)),
        }
    }
}

#[tokio::main]
async fn main() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let host = std::env::var("HOST").expect("environment variable 'HOST' is required");
    let username = std::env::var("USER").expect("environment variable 'USER' is required");

    let password = std::env::var("PASS").expect("environment variable 'PASS' is required");

    let auth = std::env::var("AUTH").expect("environment variable 'AUTH' is required");
    let auth = auth.parse::<Auth>().unwrap_or_else(|_| Auth::Password);

    let port: u16 = std::env::var("PORT")
        .ok()
        .map(|v| v.parse())
        .transpose()
        .expect("environment variable 'PORT' must be a valid u16")
        .unwrap_or(993);

    tracing::info!("Connecting to {}:{} as {}", host, port, username);

    let auth = match auth {
        Auth::Password => ImapAuth::LOGIN {
            username,
            password: SecretString::from(password),
        },
        Auth::OAuth2 => ImapAuth::XOAUTH2 {
            username,
            access_token: AccessToken::new(password),
        },
    };

    let config = ImapConnectionConfig {
        host,
        port,
        auth,
        tls: false,
        flavour: Some(ImapFlavour::Gmail),
    };

    let session = imap_connect_tls(&config)
        .await
        .expect("failed to connect to IMAP server");

    tracing::info!("Connected to IMAP server");

    let listen_config = ImapListenConfig {
        mailbox: "INBOX".to_string(),
        lookback_duration: Some(Duration::try_days(30).unwrap()),
    };

    let (mut session, mut listen) = imap_listen(session, listen_config).await.unwrap();

    tracing::info!("Listening to mailbox: {:?}", "INBOX");

    loop {
        let (returned_session, messages) = imap_idle(&mut listen, session).await.unwrap();
        session = returned_session;

        for message in messages {
            tracing::info!("Received message: {:#?}", message);
        }
    }
}
