//! BUCKAL upload session file entity
//!
//! This entity represents a file record within an upload session.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "buck_session_file")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub session_id: String,
    #[sea_orm(column_type = "Text")]
    pub file_path: String,
    pub file_size: i64,
    pub file_hash: String,
    pub file_mode: String,
    pub upload_status: String,
    #[sea_orm(nullable)]
    pub upload_reason: Option<String>,
    #[sea_orm(nullable)]
    pub blob_id: Option<String>,
    #[sea_orm(nullable)]
    pub uploaded_at: Option<DateTime>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::buck_session::Entity",
        from = "Column::SessionId",
        to = "super::buck_session::Column::SessionId"
    )]
    Session,
}

impl Related<super::buck_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new file record model
    pub fn new(
        session_id: String,
        file_path: String,
        file_size: i64,
        file_hash: String,
        file_mode: String,
        upload_status: String,
        upload_reason: Option<String>,
        blob_id: Option<String>,
    ) -> Self {
        Self {
            id: crate::entity_ext::generate_id(),
            session_id,
            file_path,
            file_size,
            file_hash,
            file_mode,
            upload_status,
            upload_reason,
            blob_id,
            uploaded_at: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

