use base64_serde::base64_serde_type;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

base64_serde_type!(Base64Standard, base64::engine::general_purpose::STANDARD);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Address {
    List(Vec<Addr>),
    Group(Vec<Group>),
}

// Define an Address struct to represent an email address with a name
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Addr {
    pub name: Option<String>,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Group {
    pub name: Option<String>,
    pub addresses: Vec<Addr>,
}

// Define an enum for content disposition of an attachment
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ContentDisposition {
    Inline,
    Attachment,
}

// Define an Attachment struct to represent an email attachment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attachment {
    pub filename: Option<String>,
    pub content_id: Option<String>,
    #[serde(with = "Base64Standard")]
    pub content: Vec<u8>,
    pub content_type: Option<String>,
    pub content_disposition: Option<ContentDisposition>,
    pub content_description: Option<String>,
    pub content_location: Option<String>,
}

// Define a Body type that can be either text or HTML
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Body {
    pub text: Option<String>,
    pub html: Option<String>,
}

// Define an Email struct to represent an email
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub envelope: Envelope,
    pub resent: Option<Envelope>,
    pub reply_to: Option<Address>,
    pub in_reply_to: Option<String>,
    pub subject: Option<String>,
    pub body_text: Vec<String>,
    pub body_html: Vec<String>,
    pub attachments: Vec<Attachment>,
    pub headers: Vec<Header>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub message_id: Option<String>,
    pub sender: Option<Address>,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub cc: Option<Address>,
    pub bcc: Option<Address>,
    pub date: Option<DateTime<Utc>>,
}

macro_rules! is_none {
    ($($field:expr),*) => {
        $($field.is_none())&&*
    };
}

impl Envelope {
    pub fn is_empty(&self) -> bool {
        is_none!(
            self.message_id,
            self.sender,
            self.from,
            self.to,
            self.cc,
            self.bcc,
            self.date
        )
    }

    pub fn into_option(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}
