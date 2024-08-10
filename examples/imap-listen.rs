use chrono::Duration;
use hark::imap::{
    imap_connect_tls, imap_idle, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig,
};

#[tokio::main]
async fn main() {
    let host = std::env::var("HOST").expect("environment variable 'HOST' is required");
    let username = std::env::var("USER").expect("environment variable 'USER' is required");
    let password = std::env::var("PASS").expect("environment variable 'PASS' is required");

    let port: u16 = std::env::var("PORT")
        .ok()
        .map(|v| v.parse())
        .transpose()
        .expect("environment variable 'PORT' must be a valid u16")
        .unwrap_or(993);

    println!("Connecting to {}:{} as {}", host, port, username);

    let session = imap_connect_tls(&ImapConnectionConfig {
        host,
        port,
        auth: ImapAuth::LOGIN { username, password },
    })
    .await
    .expect("failed to connect to IMAP server");

    let listen_config = ImapListenConfig {
        mailbox: "INBOX".to_string(),
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
