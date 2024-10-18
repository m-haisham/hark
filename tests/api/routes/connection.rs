use itertools::Itertools;

use crate::helpers::{spawn_app, TestApp};

pub async fn create_connection(app: &TestApp, connection: serde_json::Value) -> serde_json::Value {
    let response = app
        .api_client
        .post(&format!("{}/connections", app.address))
        .json(&connection)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);

    let connection = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response.");

    connection
}

pub async fn get_connection(app: &TestApp, id: &str) -> serde_json::Value {
    let response = app
        .api_client
        .get(&format!("{}/connections/{}", app.address, id))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);

    let connection = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response.");

    connection
}

pub fn new_connection(name: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "host": "localhost",
        "port": 3143,
        "tls": false,
        "username": "username",
        "auth": "password",
        "password": "password",
        "mailbox": "INBOX",
    })
}

fn update_connection() -> serde_json::Value {
    serde_json::json!({
        "host": "localhost",
        "port": 3143,
        "tls": false,
        "username": "username",
        "auth": "password",
        "password": "password",
        "mailbox": "TRASH",
    })
}

#[tokio::test]
async fn test_connnection_returns_200_for_valid_data() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client
        .post(&format!("{}/test-connection", app.address))
        .json(&new_connection("test"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_connection_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;

    let mut connection = new_connection("test");
    connection["host"] = "".into();

    // Act
    let response = app
        .api_client
        .post(&format!("{}/test-connection", app.address))
        .json(&connection)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn create_connection_returns_400_for_invalid_name() {
    // Arrange
    let app = spawn_app().await;

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
            .json(&new_connection(&name))
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

    // Act
    let response = app
        .api_client
        .post(&format!("{}/connections", app.address))
        .json(&new_connection("test"))
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
    let connection = create_connection(&app, new_connection("test")).await;
    let existing_connection = get_connection(&app, "test").await;

    // Assert
    assert_eq!(existing_connection["connection"], connection["connection"]);
}

#[tokio::test]
async fn update_connection_returns_404_for_missing_connection() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .put(&format!("{}/connections/missing", app.address))
        .json(&update_connection())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn update_connection_changes_the_existing_connection() {
    // Arrange
    let app = spawn_app().await;

    // Act
    create_connection(&app, new_connection("test")).await;

    let response = app
        .api_client
        .put(&format!("{}/connections/test", app.address))
        .json(&update_connection())
        .send()
        .await
        .expect("Failed to execute request.");

    let updated_connection = get_connection(&app, "test").await;

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(updated_connection["connection"]["mailbox"], "TRASH");
}

#[tokio::test]
async fn delete_connection_returns_404_for_missing_connection() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .delete(&format!("{}/connections/missing", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn delete_connection_returns_200_for_existing_connection() {
    // Arrange
    let app = spawn_app().await;

    // Act
    create_connection(&app, new_connection("test")).await;

    let response = app
        .api_client
        .delete(&format!("{}/connections/test", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}
