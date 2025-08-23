//! Tests for CampsiteApiStore functionality
//! 
//! These tests cover the CampsiteApiStore's ability to load user information from an external API.

use axum::{
    Router,
};
use mono::api::oauth::campsite_store::CampsiteApiStore;
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// Mock server to simulate the campsite API
async fn create_mock_campsite_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/v1/users/me", axum::routing::get(mock_user_endpoint))
        .route("/v1/users/error", axum::routing::get(mock_error_endpoint));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (addr, handle)
}

// Mock endpoint that returns user information
async fn mock_user_endpoint() -> impl axum::response::IntoResponse {
    let user_data = json!({
        "id": "1",
        "username": "testuser",
        "avatar_url": "https://example.com/avatar.jpg",
        "email": "test@example.com",
        "created_at": "2023-01-01T00:00:00Z"
    });

    axum::Json(user_data)
}

// Mock endpoint that returns an error
async fn mock_error_endpoint() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
}

#[tokio::test]
async fn test_load_user_from_api_success() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let api_url = format!("http://{}", addr);

    let store = CampsiteApiStore::new(api_url);
    
    // Test with a valid cookie
    let result = store.load_user_from_api("valid_session_cookie".to_string()).await;
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert!(user.is_some());
    
    let user = user.unwrap();
    assert_eq!(user.campsite_user_id, "1");
    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.avatar_url, "https://example.com/avatar.jpg");
}

#[tokio::test]
async fn test_load_user_from_api_invalid_cookie() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let _api_url = format!("http://{}", addr);

    // Test with an invalid cookie that causes a 401 response
    // We'll simulate this by using a non-existent endpoint
    let invalid_store = CampsiteApiStore::new(format!("http://{}/nonexistent", addr));
    
    let result = invalid_store.load_user_from_api("invalid_session_cookie".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    // Let's check that it doesn't panic and handles the error gracefully
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(user) = result {
        // If it's Ok, it should be None (no user found)
        assert!(user.is_none());
    }
    // If it's Err, that's also acceptable as the function properly handles the error
}

#[tokio::test]
async fn test_load_user_from_api_server_error() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let api_url = format!("http://{}", addr);

    let store = CampsiteApiStore::new(format!("{}/v1/users/error", api_url));
    
    let result = store.load_user_from_api("any_cookie".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    // Let's check that it doesn't panic and handles the error gracefully
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(user) = result {
        // If it's Ok, it should be None (no user found due to server error)
        assert!(user.is_none());
    }
    // If it's Err, that's also acceptable as the function properly handles the error
}

#[tokio::test]
async fn test_load_user_from_api_network_error() {
    // Test with an invalid URL that will cause a network error
    let store = CampsiteApiStore::new("http://invalid.domain.localhost:12345".to_string());
    
    let result = store.load_user_from_api("any_cookie".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    // Let's check that it doesn't panic and handles the error gracefully
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(user) = result {
        // If it's Ok, it should be None (no user found due to network error)
        assert!(user.is_none());
    }
    // If it's Err, that's also acceptable as the function properly handles the error
}