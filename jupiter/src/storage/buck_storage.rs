use std::ops::Deref;

use callisto::{buck_session, buck_session_file};
use chrono::{DateTime, Utc};
use common::errors::MegaError;
use sea_orm::prelude::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

/// File record for batch insert operations
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub file_path: String,
    pub file_size: i64,
    pub file_hash: String,
    pub file_mode: Option<String>,
    pub upload_status: String,
    pub upload_reason: Option<String>,
    pub blob_id: Option<String>,
}

#[derive(Clone)]
pub struct BuckStorage {
    pub base: BaseStorage,
}

impl Deref for BuckStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl BuckStorage {
    /// Create a new upload session
    pub async fn create_session(
        &self,
        session_id: &str,
        user_id: &str,
        repo_path: &str,
        from_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<buck_session::Model, MegaError> {
        let model = buck_session::Model::new(
            session_id.to_string(),
            user_id.to_string(),
            repo_path.to_string(),
            Some(from_hash.to_string()),
            expires_at.naive_utc(),
        );
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(res)
    }

    /// Get session by session_id
    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<buck_session::Model>, MegaError> {
        let model = buck_session::Entity::find()
            .filter(buck_session::Column::SessionId.eq(session_id))
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    /// Update session status and optionally commit_message
    pub async fn update_session_status(
        &self,
        session_id: &str,
        status: &str,
        commit_message: Option<&str>,
    ) -> Result<(), MegaError> {
        let mut update = buck_session::Entity::update_many()
            .col_expr(buck_session::Column::Status, Expr::value(status))
            .col_expr(
                buck_session::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(buck_session::Column::SessionId.eq(session_id));

        if let Some(msg) = commit_message {
            update = update.col_expr(buck_session::Column::CommitMessage, Expr::value(msg));
        }

        update.exec(self.get_connection()).await?;
        Ok(())
    }

    /// Batch insert file records
    pub async fn batch_insert_files(
        &self,
        session_id: &str,
        records: Vec<FileRecord>,
    ) -> Result<(), MegaError> {
        if records.is_empty() {
            return Ok(());
        }

        let models: Vec<buck_session_file::ActiveModel> = records
            .into_iter()
            .map(|record| {
                let model = buck_session_file::Model::new(
                    session_id.to_string(),
                    record.file_path,
                    record.file_size,
                    record.file_hash,
                    record.file_mode,
                    record.upload_status,
                    record.upload_reason,
                    record.blob_id,
                );
                model.into_active_model()
            })
            .collect();

        buck_session_file::Entity::insert_many(models)
            .exec(self.get_connection())
            .await?;

        Ok(())
    }

    /// Get a pending file by session_id and file_path
    pub async fn get_pending_file(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<Option<buck_session_file::Model>, MegaError> {
        let model = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::FilePath.eq(file_path))
            .filter(buck_session_file::Column::UploadStatus.eq("pending"))
            .filter(buck_session_file::Column::UploadReason.is_not_null())
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    /// Mark a file as uploaded
    pub async fn mark_file_uploaded(
        &self,
        session_id: &str,
        file_path: &str,
        blob_id: &str,
    ) -> Result<u64, MegaError> {
        let result = buck_session_file::Entity::update_many()
            .col_expr(
                buck_session_file::Column::UploadStatus,
                Expr::value("uploaded"),
            )
            .col_expr(buck_session_file::Column::BlobId, Expr::value(blob_id))
            .col_expr(
                buck_session_file::Column::UploadedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::FilePath.eq(file_path))
            .filter(buck_session_file::Column::UploadStatus.eq("pending"))
            .exec(self.get_connection())
            .await?;

        Ok(result.rows_affected)
    }

    /// Count pending files
    pub async fn count_pending_files(&self, session_id: &str) -> Result<u64, MegaError> {
        let count = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::UploadStatus.eq("pending"))
            .filter(buck_session_file::Column::UploadReason.is_not_null())
            .count(self.get_connection())
            .await?;
        Ok(count)
    }

    /// Get all files for a session
    pub async fn get_all_files(
        &self,
        session_id: &str,
    ) -> Result<Vec<buck_session_file::Model>, MegaError> {
        let files = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .all(self.get_connection())
            .await?;
        Ok(files)
    }

    /// Get uploaded files only
    pub async fn get_uploaded_files(
        &self,
        session_id: &str,
    ) -> Result<Vec<buck_session_file::Model>, MegaError> {
        let files = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::UploadStatus.eq("uploaded"))
            .all(self.get_connection())
            .await?;
        Ok(files)
    }

    /// Delete expired sessions
    pub async fn delete_expired_sessions(&self, retention_days: u32) -> Result<u64, MegaError> {
        let now = Utc::now().naive_utc();
        let retention_cutoff = now - chrono::Duration::days(retention_days as i64);

        let result = buck_session::Entity::delete_many()
            .filter(
                Condition::any()
                    //Expired and not completed
                    .add(
                        Condition::all()
                            .add(buck_session::Column::ExpiresAt.lt(now))
                            .add(buck_session::Column::Status.ne("completed")),
                    )
                    //Completed but older than retention period
                    .add(
                        Condition::all()
                            .add(buck_session::Column::Status.eq("completed"))
                            .add(buck_session::Column::CreatedAt.lt(retention_cutoff)),
                    ),
            )
            .exec(self.get_connection())
            .await?;
        Ok(result.rows_affected)
    }

    /// Delete file records for a specific session
    pub async fn delete_session_files(&self, session_id: &str) -> Result<u64, MegaError> {
        let result = buck_session_file::Entity::delete_many()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .exec(self.get_connection())
            .await?;
        Ok(result.rows_affected)
    }
}
