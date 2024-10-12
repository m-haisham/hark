use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImapFlavour {
    Gmail,
}

impl ImapFlavour {
    pub fn from_host(host: &str) -> Option<Self> {
        match host {
            "imap.gmail.com" => Some(Self::Gmail),
            _ => None,
        }
    }
}

impl Display for ImapFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImapFlavour::Gmail => write!(f, "google"),
        }
    }
}
