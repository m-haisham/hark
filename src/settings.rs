use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use serde::Deserialize;
use url::Url;

use crate::connection::types::{Connection, ConnectionId};

pub fn get_config(file: &str) -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine current directory");

    let settings: Settings = config::Config::builder()
        .add_source(config::File::from(base_path.join(file)))
        .build()?
        .try_deserialize()?;

    Ok(settings)
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    #[serde(default)]
    pub lazy: LazySettings,
    #[serde(default)]
    pub connections: HashMap<ConnectionId, Connection>,
    #[serde(default)]
    pub anchor: AnchorSettings,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LazySettings {
    #[serde(default = "default_timeout")]
    #[serde(deserialize_with = "duration_millis")]
    pub timeout: Duration,
    #[serde(default = "default_heartbeat")]
    #[serde(deserialize_with = "duration_millis")]
    pub heartbeat: Duration,
    /// Maximum number of fetch requests per session, if not set, it will be unlimited.
    /// This is mostly used for testing purposes, hence it is not exposed in the config file.
    #[serde(skip)]
    pub max_fetch_count: Option<usize>,
}

impl Default for LazySettings {
    fn default() -> Self {
        LazySettings {
            timeout: default_timeout(),
            heartbeat: default_heartbeat(),
            max_fetch_count: None,
        }
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_heartbeat() -> Duration {
    Duration::from_secs(30)
}

fn duration_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ms = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(ms))
}

#[derive(Deserialize, Debug, Clone)]
pub struct AnchorSettings {
    #[serde(default)]
    pub fetch_url: Option<Url>,
    #[serde(default = "default_callback_url")]
    pub callback_url: Url,
    #[serde(default)]
    pub ping: bool,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl Default for AnchorSettings {
    fn default() -> Self {
        AnchorSettings {
            fetch_url: None,
            callback_url: default_callback_url(),
            ping: true,
            headers: BTreeMap::new(),
        }
    }
}

fn default_callback_url() -> Url {
    Url::parse("http://localhost:8080").unwrap()
}
