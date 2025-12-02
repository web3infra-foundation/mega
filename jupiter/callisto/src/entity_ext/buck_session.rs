use crate::{buck_session, entity_ext::generate_id};
use sea_orm::entity::prelude::*;

impl buck_session::Model {
    /// Create a new session model
    pub fn new(
        session_id: String,
        user_id: String,
        repo_path: String,
        from_hash: Option<String>,
        expires_at: DateTime,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            session_id,
            user_id,
            repo_path,
            status: "created".to_string(),
            commit_message: None,
            from_hash,
            expires_at,
            created_at: now,
            updated_at: now,
        }
    }
}
