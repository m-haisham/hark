use anyhow::Context;
use chrono::Duration;
use hark::{
    imap::{
        imap_connect, imap_idle, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig,
    },
    settings::{self, ConnectionSetting},
};
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let settings = settings::get_config(".").expect("Failed to read config");

    let mut tasks = JoinSet::new();

    for (key, connection) in settings.connections {
        tracing::info!("Spawning task for connection: {}", key);
        tasks.spawn(listen_to_connection(connection));
    }

    while let Some(res) = tasks.join_next().await {
        let out = res?;

        match out {
            Ok(_) => tracing::info!("Listening task completed successfully"),
            Err(e) => tracing::error!("Listening task failed: {:?}", e),
        }
    }

    Ok(())
}

#[tracing::instrument]
async fn listen_to_connection(connection: ConnectionSetting) -> anyhow::Result<()> {
    tracing::trace!(
        "Connecting to IMAP server: {}:{}",
        connection.host,
        connection.port
    );

    let auth = match connection.auth {
        settings::ConnectionAuth::Password { password } => ImapAuth::LOGIN {
            username: connection.username,
            password,
        },
        settings::ConnectionAuth::Xoauth2 { token } => ImapAuth::XOAUTH2 {
            username: connection.username,
            access_token: token,
        },
    };

    let imap_connection = ImapConnectionConfig {
        host: connection.host,
        port: connection.port,
        auth,
    };

    let session = imap_connect(&imap_connection)
        .await
        .context("Failed to connect to IMAP server")?;

    let listen_config = ImapListenConfig {
        mailbox: connection.mailbox,
        lookback_duration: Some(Duration::try_days(30).unwrap()),
    };

    let (mut session, mut listen) = imap_listen(session, listen_config)
        .await
        .context("Failed to start listening to IMAP server")?;

    loop {
        let (returned_session, messages) = imap_idle(&mut listen, session)
            .await
            .context("Failed to idle")?;

        session = returned_session;

        for message in messages {
            println!("Received message: {:#?}", message);
        }
    }
}
