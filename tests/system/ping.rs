use hark::{anchor::Anchor, settings::AnchorSettings};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::matchers::callback_type;

pub struct TestAnchor {
    pub mock_server: wiremock::MockServer,
    pub anchor: Anchor,
}

impl TestAnchor {
    pub async fn new() -> Self {
        let mock_server = wiremock::MockServer::start().await;

        let anchor = Anchor::new(
            reqwest::Client::new(),
            AnchorSettings {
                callback_url: url::Url::parse(&mock_server.uri())
                    .expect("Failed to parse mock server URL")
                    .join("/callback")
                    .expect("Failed to join callback path"),
                ..Default::default()
            },
        );

        Self {
            mock_server,
            anchor,
        }
    }
}

#[tokio::test]
async fn ping_callback_must_succeed_when_200_returned() {
    let test_anchor = TestAnchor::new().await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("ping"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_anchor.mock_server)
        .await;

    let result = test_anchor.anchor.ping().await;
    assert!(result.is_ok(), "Ping failed: {:?}", result);
}

#[tokio::test]
async fn ping_callback_must_fail_when_200_is_not_returned() {
    let anchor = TestAnchor::new().await;

    Mock::given(method("POST"))
        .and(path("/callback"))
        .and(callback_type("ping"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&anchor.mock_server)
        .await;

    let result = anchor.anchor.ping().await;
    assert!(result.is_err(), "Ping succeeded: {:?}", result);
}
