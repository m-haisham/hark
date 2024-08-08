use itertools::Itertools;

use crate::helpers::spawn_app;

#[tokio::test]
async fn create_connection_returns_400_for_invalid_name() {
    // Arrange
    let app = spawn_app().await;

    fn new_connection_with_name(name: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "host": "localhost",
            "port": 5432,
            "username": "postgres",
            "auth": "password",
            "password": "password",
            "mailbox": "INBOX",
        })
    }

    let test_cases = vec!["", " ", "1a", "a b"]
        .into_iter()
        .map(ToString::to_string)
        .chain(["a".repeat(21)])
        .collect_vec();

    for name in test_cases {
        // Act
        let response = app
            .api_client
            .post(&format!("{}/connections", app.address))
            .json(&new_connection_with_name(&name))
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(response.status().as_u16(), 400, "name: {}", name);
    }
}
