use chrono::Duration;
use hark::imap::{imap_connect, imap_listen, ImapAuth, ImapConnectionConfig, ImapListenConfig};

fn main() {
    let host = std::env::var("HOST").expect("environment variable 'HOST' is required");
    let username = std::env::var("USER").expect("environment variable 'USER' is required");
    let password = std::env::var("PASS").expect("environment variable 'PASS' is required");

    let port: u16 = std::env::var("PORT")
        .ok()
        .map(|v| v.parse())
        .transpose()
        .expect("environment variable 'PORT' must be a valid u16")
        .unwrap_or(993);

    let session = imap_connect(&ImapConnectionConfig {
        host,
        port,
        auth: ImapAuth::LOGIN { username, password },
    })
    .unwrap();

    let listen_config = ImapListenConfig {
        mailbox: "INBOX".to_string(),
        lookback_duration: Some(Duration::try_days(30).unwrap()),
    };

    for result in imap_listen(session, listen_config).unwrap() {
        match result {
            Ok(emails) => {
                for email in emails {
                    println!("{email:?}");
                }
            }
            Err(error) => {
                eprintln!("{error}");
                break;
            }
        }
    }
}
