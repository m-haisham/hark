use std::collections::HashMap;

use serde::Deserialize;

pub fn get_config(dir: &str) -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine current directory");
    let config_dir = base_path.join(dir);

    let settings: Settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("config.toml")))
        .build()?
        .try_deserialize()?;

    Ok(settings)
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Settings {
    pub connections: HashMap<String, ConnectionSetting>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ConnectionSetting {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(flatten)]
    pub auth: ConnectionAuth,
    pub mailbox: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum ConnectionAuth {
    Password { password: String },
    Xoauth2 { token: String },
}

impl Default for ConnectionAuth {
    fn default() -> Self {
        ConnectionAuth::Password {
            password: String::new(),
        }
    }
}
