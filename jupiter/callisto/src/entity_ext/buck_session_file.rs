use crate::{buck_session_file, entity_ext::generate_id};

impl buck_session_file::Model {
    /// Create a new file record model
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        file_path: String,
        file_size: i64,
        file_hash: String,
        file_mode: Option<String>,
        upload_status: String,
        upload_reason: Option<String>,
        blob_id: Option<String>,
    ) -> Self {
        Self {
            id: generate_id(),
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
