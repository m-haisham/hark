use wiremock::{Match, Request};

pub fn callback_type(r#type: &'static str) -> CallbackType {
    CallbackType(r#type)
}

pub struct CallbackType(&'static str);

#[derive(Debug, serde::Deserialize)]
pub struct Body {
    pub r#type: String,
}

impl Match for CallbackType {
    fn matches(&self, request: &Request) -> bool {
        match request.body_json::<Body>() {
            Ok(body) => body.r#type == self.0,
            Err(_) => false,
        }
    }
}
