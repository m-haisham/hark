use std::collections::{BTreeMap, HashMap};

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
