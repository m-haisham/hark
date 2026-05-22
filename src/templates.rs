use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct Index {
    pub style: FrontendStyle,
}

pub enum FrontendStyle {
    Filesystem(String),
    Embedded(&'static str),
}
