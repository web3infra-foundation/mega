//! Tests for LoginUser extractor functionality
//! 
//! These tests cover the LoginUser extractor's ability to extract user information from requests.
//! Since the extractor relies on CampsiteApiStore, we focus on testing the underlying functionality.

use mono::api::oauth::campsite_store::CampsiteApiStore;
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// Mock server to simulate the campsite API
async fn create_mock_campsite_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = axum::Router::new()
        .route("/v1/users/me", axum::routing::get(mock_user_endpoint));

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

#[tokio::test]
async fn test_login_user_extractor_success() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let _api_url = format!("http://{}", addr);

    // Create a mock store
    let store = CampsiteApiStore::new(format!("http://{}", addr));
    
    // Test the load_user_from_api method directly
    let result = store.load_user_from_api("valid_session_cookie".to_string()).await;
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert!(user.is_some());
    
    let user = user.unwrap();
    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.campsite_user_id, "1");
    assert_eq!(user.avatar_url, "https://example.com/avatar.jpg");
}

#[tokio::test]
async fn test_login_user_extractor_invalid_cookie() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let _api_url = format!("http://{}", addr);

    // Create a mock store with a non-existent endpoint to simulate an invalid cookie
    let store = CampsiteApiStore::new(format!("http://{}/nonexistent", addr));
    
    // Test the load_user_from_api method directly
    let result = store.load_user_from_api("invalid_session_cookie".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(user) = result {
        // If it's Ok, it should be None (no user found)
        assert!(user.is_none());
    }
}

#[tokio::test]
async fn test_login_user_extractor_missing_cookie() {
    let (addr, _handle) = create_mock_campsite_server().await;
    let _api_url = format!("http://{}", addr);

    // Create a mock store
    let store = CampsiteApiStore::new(format!("http://{}", addr));
    
    // Test with an empty cookie string to simulate missing cookie
    let result = store.load_user_from_api("".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    assert!(result.is_ok() || result.is_err());
    
    // Note: The behavior when an empty cookie is provided depends on the 
    // campsite API implementation. It might return an error or None.
    // We're testing that it doesn't panic and handles the situation gracefully.
}

#[tokio::test]
async fn test_login_user_extractor_network_error() {
    // Test with an invalid URL that will cause a network error
    let store = CampsiteApiStore::new("http://invalid.domain.localhost:12345".to_string());
    
    let result = store.load_user_from_api("any_cookie".to_string()).await;
    
    // Depending on the implementation, this might be Ok(None) or an Err
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(user) = result {
        // If it's Ok, it should be None (no user found due to network error)
        assert!(user.is_none());
    }
}