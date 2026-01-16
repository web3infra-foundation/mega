use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewGpgRequest {
    pub gpg_content: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RemoveGpgRequest {
    pub key_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GpgKey {
    pub user_id: String,
    pub key_id: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
