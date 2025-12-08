//! Buck upload session management storage layer.
//!
//! This module provides CRUD operations for Buck upload sessions and associated file records.
//! Sessions track the lifecycle of bulk file uploads, including status transitions from
//! `created` → `manifest_uploaded` → `uploading` → `completed`, and handle file-level upload tracking.

use std::ops::Deref;

use callisto::{buck_session, buck_session_file};
use chrono::{DateTime, Utc};
use common::errors::MegaError;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

/// Buck session status constants.
///
/// These constants define the valid states of a Buck upload session lifecycle.
pub mod session_status {
    /// Session just created, no manifest uploaded yet
    pub const CREATED: &str = "created";
    /// Manifest has been uploaded and validated
    pub const MANIFEST_UPLOADED: &str = "manifest_uploaded";
    /// Files are being uploaded
    pub const UPLOADING: &str = "uploading";
    /// All files uploaded and session finalized
    pub const COMPLETED: &str = "completed";
}

/// File upload status constants.
///
/// These constants define the valid states of individual file uploads within a session.
pub mod upload_status {
    /// File waiting to be uploaded
    pub const PENDING: &str = "pending";
    /// File successfully uploaded
    pub const UPLOADED: &str = "uploaded";
}

/// File upload reason constants.
///
/// These constants explain why a file needs to be uploaded.
pub mod upload_reason {
    /// File is new (not in base commit)
    pub const NEW: &str = "new";
    /// File has been modified from base commit
    pub const MODIFIED: &str = "modified";
}

/// File record for batch insert operations.
///
/// Represents metadata about a file in a Buck upload session, tracking
/// its upload status, hash, size, and associated blob ID.
#[derive(Debug, Clone)]
pub struct FileRecord {
    /// Repository-relative file path
    pub file_path: String,
    /// File size in bytes
    pub file_size: i64,
    /// Content hash (typically SHA-1)
    pub file_hash: String,
    /// Git file mode (e.g., "100644", "100755")
    pub file_mode: Option<String>,
    /// Current upload status (use [`upload_status`] constants)
    pub upload_status: String,
    /// Reason for upload (use [`upload_reason`] constants)
    pub upload_reason: Option<String>,
    /// Database blob ID after upload
    pub blob_id: Option<String>,
}

/// Storage layer for Buck upload sessions.
///
/// Provides CRUD operations for managing Buck upload sessions and their associated
/// file records. Supports session lifecycle management, file tracking, and cleanup
/// of expired sessions.
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
    /// Create a new Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    /// * `user_id` - User creating the session
    /// * `repo_path` - Repository path
    /// * `from_hash` - Base commit hash for diff calculation
    /// * `expires_at` - Session expiration timestamp
    ///
    /// # Returns
    /// The created session model
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

    /// Retrieve a Buck upload session by its ID.
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    ///
    /// # Returns
    /// - `Ok(Some(model))` if session exists
    /// - `Ok(None)` if session not found
    /// - `Err(_)` on database error
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

    /// Update a Buck upload session status and optionally commit message.
    ///
    /// # Arguments
    /// * `session_id` - The session to update
    /// * `status` - New status (use [`session_status`] constants)
    /// * `commit_message` - Optional commit message to save
    ///
    /// # Returns
    /// Returns `Ok(())` on success
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

    /// Batch insert file records for a Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - The session to add files to
    /// * `records` - Vector of file records to insert
    ///
    /// # Returns
    /// Returns `Ok(())` on success. If records is empty, returns immediately without database operation.
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

        // Use ON CONFLICT DO NOTHING to ensure idempotency
        // This allows safe retries: already-inserted records are silently skipped
        buck_session_file::Entity::insert_many(models)
            .on_conflict(
                OnConflict::columns(vec![
                    buck_session_file::Column::SessionId,
                    buck_session_file::Column::FilePath,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(self.get_connection())
            .await?;

        Ok(())
    }

    /// Get a pending file by session_id and file_path.
    ///
    /// # Arguments
    /// * `session_id` - The session containing the file
    /// * `file_path` - Repository-relative path of the file
    ///
    /// # Returns
    /// - `Ok(Some(model))` if pending file found
    /// - `Ok(None)` if file not found or not pending
    /// - `Err(_)` on database error
    pub async fn get_pending_file(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<Option<buck_session_file::Model>, MegaError> {
        let model = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::FilePath.eq(file_path))
            .filter(buck_session_file::Column::UploadStatus.eq(upload_status::PENDING))
            .filter(buck_session_file::Column::UploadReason.is_not_null())
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    /// Mark a file as successfully uploaded within a Buck upload session.
    ///
    /// Updates the file record with:
    /// - Status changed from `pending` to `uploaded`
    /// - `blob_id` populated with the database blob reference
    /// - `uploaded_at` timestamp set to current time
    ///
    /// # Arguments
    /// * `session_id` - The session containing the file
    /// * `file_path` - Repository-relative path of the file
    /// * `blob_id` - Database blob ID where content was stored
    ///
    /// # Returns
    /// Number of rows affected (should be 1 on success, 0 if file not found or already uploaded)
    pub async fn mark_file_uploaded(
        &self,
        session_id: &str,
        file_path: &str,
        blob_id: &str,
    ) -> Result<u64, MegaError> {
        let result = buck_session_file::Entity::update_many()
            .col_expr(
                buck_session_file::Column::UploadStatus,
                Expr::value(upload_status::UPLOADED),
            )
            .col_expr(buck_session_file::Column::BlobId, Expr::value(blob_id))
            .col_expr(
                buck_session_file::Column::UploadedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::FilePath.eq(file_path))
            .filter(buck_session_file::Column::UploadStatus.eq(upload_status::PENDING))
            .exec(self.get_connection())
            .await?;

        Ok(result.rows_affected)
    }

    /// Count pending files in a Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - The session to count pending files for
    ///
    /// # Returns
    /// Number of files with status `pending` and non-null `upload_reason`
    pub async fn count_pending_files(&self, session_id: &str) -> Result<u64, MegaError> {
        let count = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::UploadStatus.eq(upload_status::PENDING))
            .filter(buck_session_file::Column::UploadReason.is_not_null())
            .count(self.get_connection())
            .await?;
        Ok(count)
    }

    /// Get all files for a Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - The session to retrieve files from
    ///
    /// # Returns
    /// Vector of all file records regardless of upload status
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

    /// Get uploaded files only for a Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - The session to retrieve uploaded files from
    ///
    /// # Returns
    /// Vector of file records with status `uploaded`
    pub async fn get_uploaded_files(
        &self,
        session_id: &str,
    ) -> Result<Vec<buck_session_file::Model>, MegaError> {
        let files = buck_session_file::Entity::find()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .filter(buck_session_file::Column::UploadStatus.eq(upload_status::UPLOADED))
            .all(self.get_connection())
            .await?;
        Ok(files)
    }

    /// Delete all expired Buck upload sessions according to retention policy.
    ///
    /// # Deletion Rules
    ///
    /// A session is deleted if:
    /// 1. **Expired incomplete**: `expires_at < now` AND `status != "completed"`
    /// 2. **Old completed**: `status == "completed"` AND `created_at < (now - retention_days)`
    ///
    /// # Arguments
    /// * `retention_days` - Number of days to keep completed sessions
    ///
    /// # Returns
    /// Number of sessions deleted
    pub async fn delete_expired_sessions(&self, retention_days: u32) -> Result<u64, MegaError> {
        let now = Utc::now().naive_utc();
        let retention_cutoff = now - chrono::Duration::days(retention_days as i64);

        let result = buck_session::Entity::delete_many()
            .filter(
                Condition::any()
                    // Expired and not completed
                    .add(
                        Condition::all()
                            .add(buck_session::Column::ExpiresAt.lt(now))
                            .add(buck_session::Column::Status.ne(session_status::COMPLETED)),
                    )
                    // Completed but older than retention period
                    .add(
                        Condition::all()
                            .add(buck_session::Column::Status.eq(session_status::COMPLETED))
                            .add(buck_session::Column::CreatedAt.lt(retention_cutoff)),
                    ),
            )
            .exec(self.get_connection())
            .await?;
        Ok(result.rows_affected)
    }

    /// Delete all file records for a specific Buck upload session.
    ///
    /// # Arguments
    /// * `session_id` - The session whose files should be deleted
    ///
    /// # Returns
    /// Number of file records deleted
    pub async fn delete_session_files(&self, session_id: &str) -> Result<u64, MegaError> {
        let result = buck_session_file::Entity::delete_many()
            .filter(buck_session_file::Column::SessionId.eq(session_id))
            .exec(self.get_connection())
            .await?;
        Ok(result.rows_affected)
    }
}
