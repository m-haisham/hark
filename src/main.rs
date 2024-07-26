use anyhow::Context;
use chrono::Duration;
use hark::{
    imap::{
        imap_connect, imap_idle, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig,
    },
    settings::{self, ConnectionSetting},
    telemetry::{self, init_subscriber},
};
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = telemetry::get_subscriber("info".into());
    init_subscriber(subscriber);

    let settings = settings::get_config(".").expect("Failed to read config");

    let mut tasks = JoinSet::new();

    for connection in settings.connections {
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

async fn listen_to_connection(connection: ConnectionSetting) -> anyhow::Result<()> {
    println!(
        "Connecting to {}:{} as {}",
        connection.host, connection.port, connection.username
    );

    let imap_connection = ImapConnectionConfig {
        host: connection.host,
        port: connection.port,
        auth: match connection.auth {
            settings::ConnectionAuth::Password { password } => ImapAuth::LOGIN {
                username: connection.username,
                password,
            },
            settings::ConnectionAuth::Xoauth2 { token } => ImapAuth::XOAUTH2 {
                username: connection.username,
                access_token: token,
            },
        },
    };

    let session = imap_connect(&imap_connection)
        .await
        .context("Failed to connect to IMAP server")?;

    let listen_config = ImapListenConfig {
        mailbox: connection.mailbox,
        lookback_duration: Some(Duration::try_days(30).unwrap()),
    };

    let (mut session, mut listen) = imap_listen(session, listen_config).await.unwrap();

    println!("Listening to mailbox: {:?}", "INBOX");

    loop {
        let (returned_session, messages) = imap_idle(&mut listen, session).await.unwrap();
        session = returned_session;

        for message in messages {
            println!("Received message: {:#?}", message);
        }
    }
}
