use std::collections::HashMap;

use serde::Deserialize;

use crate::connection::types::Connection;

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
    pub connections: HashMap<String, Connection>,
}
