use async_imap::{types::Capability, Client, Session};
use eyre::Context;
use oauth2::AccessToken;
use secrecy::{ExposeSecret, SecretString};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::{
    rustls::{pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};

use crate::connection::types::ImapFlavour;
use eyre::eyre;
use std::{fmt::Debug, sync::Arc};

use super::session::TlsConnection;

#[derive(Debug)]
pub struct ImapConnectionConfig {
    pub host: String,
    pub port: u16,
    pub auth: ImapAuth,
    pub tls: bool,
    pub flavour: Option<ImapFlavour>,
}

#[derive(Debug)]
pub enum ImapAuth {
    LOGIN {
        username: String,
        password: SecretString,
    },
    XOAUTH2 {
        username: String,
        access_token: AccessToken,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ImapError {
    #[error("{0}")]
    Imap(#[from] async_imap::error::Error),

    #[error("Imap server does not define the capability: {1}")]
    LackingCapability(Capability, String),

    #[error("{0}")]
    AuthFailed(#[source] async_imap::error::Error),
}

struct XOAuth2Authenticator<'a> {
    user: &'a str,
    access_token: &'a str,
}

impl async_imap::Authenticator for XOAuth2Authenticator<'_> {
    type Response = String;

    fn process(&mut self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[tracing::instrument(
    name = "IMAP Connect with TLS",
    skip(config),
    fields(host = %config.host.as_str(), port = %config.port)
)]
pub async fn imap_connect_tls(
    config: &ImapConnectionConfig,
) -> eyre::Result<Session<TlsConnection>> {
    let mut root_cert_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().unwrap() {
        root_cert_store.add(cert).unwrap();
    }

    let rustls_config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let addr = (config.host.as_str(), config.port);
    let tcp_stream = TcpStream::connect(addr)
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err_with(|| {
            format!(
                "Failed to connect to IMAP server at {}:{}",
                config.host, config.port
            )
        })?;

    let connector = TlsConnector::from(Arc::new(rustls_config));

    let server_name = ServerName::try_from(config.host.as_str())?.to_owned();
    let tls_stream = connector.connect(server_name, tcp_stream).await?;

    let client = create_client(config, tls_stream).await?;

    imap_auth(client, &config.auth)
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to authenticate with IMAP server")
}

#[tracing::instrument(
    name = "IMAP Connect with TCP",
    skip(config),
    fields(host = %config.host.as_str(), port = %config.port)
)]
pub async fn imap_connect_tcp(config: &ImapConnectionConfig) -> eyre::Result<Session<TcpStream>> {
    let addr = (config.host.as_str(), config.port);
    let stream = TcpStream::connect(addr)
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err_with(|| {
            format!(
                "Failed to connect to IMAP server at {}:{}",
                config.host, config.port
            )
        })?;

    let client = create_client(config, stream).await?;

    imap_auth(client, &config.auth)
        .await
        .map_err(|e| eyre::eyre!(e))
        .wrap_err("Failed to authenticate with IMAP server")
}

#[tracing::instrument(
    name = "IMAP Create Client",
    skip_all,
    fields(
        host = %config.host.as_str(),
        port = %config.port,
        flavour = ?config.flavour
    )
)]
async fn create_client<T>(config: &ImapConnectionConfig, stream: T) -> eyre::Result<Client<T>>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let mut client = async_imap::Client::new(stream);

    if let Some(ImapFlavour::Gmail) = config.flavour {
        tracing::info!("Gmail IMAP flavour detected, receiving greeting");

        client
            .read_response()
            .await
            .transpose()
            .ok_or_else(|| eyre!("Failed to read greeting response from gmail IMAP server"))?
            .map_err(|e| eyre!(e))
            .wrap_err("Failed to read response")?;
    }

    Ok(client)
}

#[tracing::instrument(name = "IMAP Authenticate", skip(client))]
pub async fn imap_auth<T>(client: Client<T>, auth: &ImapAuth) -> Result<Session<T>, ImapError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match &auth {
        ImapAuth::LOGIN { username, password } => client
            .login(username, password.expose_secret())
            .await
            .map_err(parse_auth_error),
        ImapAuth::XOAUTH2 {
            username,
            access_token,
        } => {
            let cred = XOAuth2Authenticator {
                user: username,
                access_token: access_token.secret(),
            };

            client
                .authenticate("XOAUTH2", cred)
                .await
                .map_err(parse_auth_error)
        }
    }
}

pub fn parse_auth_error<T>((error, _client): (async_imap::error::Error, Client<T>)) -> ImapError
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match error {
        async_imap::error::Error::No(ref e) if e.contains("AUTHENTICATIONFAILED") => {
            ImapError::AuthFailed(error)
        }
        _ => ImapError::Imap(error),
    }
}

#[tracing::instrument(skip_all)]
pub async fn check_capability<T>(
    session: &mut Session<T>,
    capability: Capability,
) -> Result<(), ImapError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    if !session.capabilities().await?.has(&capability) {
        let display = match &capability {
            Capability::Imap4rev1 => "IMAP4rev1".to_string(),
            Capability::Auth(v) => format!("AUTH={}", v),
            Capability::Atom(v) => format!("{}", v),
        };

        return Err(ImapError::LackingCapability(capability, display));
    }

    Ok(())
}
