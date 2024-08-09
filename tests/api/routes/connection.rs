use itertools::Itertools;

use crate::helpers::{spawn_app, TestApp};

async fn create_connection(app: &TestApp, connection: serde_json::Value) -> serde_json::Value {
    let response = app
        .api_client
        .post(&format!("{}/connections", app.address))
        .json(&connection)
        .send()
        .await
        .expect("Failed to execute request.");

    response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response.")
}

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

#[tokio::test]
async fn create_connection_returns_connection_for_valid_data() {
    // Arrange
    let app = spawn_app().await;

    let connection = serde_json::json!({
        "name": "test",
        "host": "localhost",
        "port": 5432,
        "username": "postgres",
        "auth": "password",
        "password": "password",
        "mailbox": "INBOX",
    });

    // Act
    let response = app
        .api_client
        .post(&format!("{}/connections", app.address))
        .json(&connection)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);

    let data = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response.");
    assert_eq!(data["id"], "test");
}

#[tokio::test]
async fn list_connections_returns_200() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .get(&format!("{}/connections", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn get_connection_returns_404_for_missing_connection() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .get(&format!("{}/connections/missing", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn get_connection_returns_200_for_existing_connection() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let connection = create_connection(
        &app,
        serde_json::json!({
            "name": "test",
            "host": "localhost",
            "port": 5432,
            "username": "postgres",
            "auth": "password",
            "password": "password",
            "mailbox": "INBOX",
        }),
    )
    .await;

    let response = app
        .api_client
        .get(&format!("{}/connections/test", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    let data = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response.");

    assert_eq!(data, connection);
}
