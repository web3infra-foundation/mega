//! Tests for session management functionality
//!
//! These tests cover the basic session management using tower-sessions.

use std::sync::Arc;

use tower_sessions::{MemoryStore, Session};

#[tokio::test]
async fn test_session_creation_and_retrieval() {
    let store = MemoryStore::default();

    // Create a new session
    let session = Session::new(None, Arc::new(store.clone()), None);

    // Insert some data into the session
    let user_data = serde_json::json!({
        "id": 1,
        "username": "testuser",
        "email": "test@example.com",
        "name": "Test User"
    });

    session.insert("user", &user_data).await.unwrap();

    // Save the session
    session.save().await.unwrap();

    // Get the session ID
    let session_id = session.id().unwrap();

    // Create a new session with the same ID to simulate retrieval
    let retrieved_session = Session::new(Some(session_id), Arc::new(store.clone()), None);

    // Retrieve the data from the session
    let retrieved_user: Option<serde_json::Value> = retrieved_session.get("user").await.unwrap();

    assert!(retrieved_user.is_some());
    let user = retrieved_user.unwrap();
    assert_eq!(user["username"], "testuser");
    assert_eq!(user["email"], "test@example.com");
}

#[tokio::test]
async fn test_session_clearing() {
    let store = MemoryStore::default();

    // Create a new session
    let session = Session::new(None, Arc::new(store.clone()), None);

    // Insert some data into the session
    let user_data = serde_json::json!({
        "id": 1,
        "username": "testuser",
        "email": "test@example.com",
        "name": "Test User"
    });

    session.insert("user", &user_data).await.unwrap();

    // Save the session
    session.save().await.unwrap();

    // Get the session ID
    let session_id = session.id().unwrap();

    // Clear the session
    session.flush().await.unwrap();

    // Try to retrieve the data from the session - should be None
    let retrieved_session = Session::new(Some(session_id), Arc::new(store.clone()), None);
    let retrieved_user: Option<serde_json::Value> = retrieved_session.get("user").await.unwrap();

    assert!(retrieved_user.is_none());
}

#[tokio::test]
async fn test_session_persistence() {
    let store = MemoryStore::default();

    // Create a new session
    let session = Session::new(None, Arc::new(store.clone()), None);

    // Insert some data into the session
    let user_data = serde_json::json!({
        "id": 1,
        "username": "testuser",
        "email": "test@example.com",
        "name": "Test User"
    });

    session.insert("user", &user_data).await.unwrap();

    // Save the session
    session.save().await.unwrap();

    // Get the session ID
    let session_id = session.id().unwrap();

    // Make multiple requests with the same session ID
    for _ in 0..3 {
        let retrieved_session = Session::new(Some(session_id), Arc::new(store.clone()), None);
        let retrieved_user: Option<serde_json::Value> =
            retrieved_session.get("user").await.unwrap();

        assert!(retrieved_user.is_some());
        let user = retrieved_user.unwrap();
        assert_eq!(user["username"], "testuser");
        assert_eq!(user["email"], "test@example.com");
    }
}

#[tokio::test]
async fn test_session_id_generation() {
    let store = MemoryStore::default();

    // Create a new session
    let session = Session::new(None, Arc::new(store.clone()), None);

    // Initially, the session ID might be None
    let _initial_session_id = session.id();
    // assert!(_initial_session_id.is_some()); // This might be None initially

    // Insert some data and save the session
    let user_data = serde_json::json!({
        "id": 1,
        "username": "testuser",
        "email": "test@example.com",
        "name": "Test User"
    });

    session.insert("user", &user_data).await.unwrap();
    session.save().await.unwrap();

    // After saving, the session should have an ID
    let session_id = session.id();
    assert!(session_id.is_some());
}
