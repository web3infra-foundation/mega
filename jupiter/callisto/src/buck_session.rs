//! BUCKAL upload session entity
//!
//! This entity represents an upload session for batch file uploads.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buck_session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub session_id: String,
    pub user_id: String,
    #[sea_orm(column_type = "Text")]
    pub repo_path: String,
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub commit_message: Option<String>,
    #[sea_orm(nullable)]
    pub from_hash: Option<String>,
    pub expires_at: DateTime,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::buck_session_file::Entity")]
    Files,
}

impl Related<super::buck_session_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
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
            id: crate::entity_ext::generate_id(),
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

