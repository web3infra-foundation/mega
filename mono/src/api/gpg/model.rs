use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewGpgRequest {
    pub user_id: i64,
    pub gpg_content: String,
    pub expires_days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RemoveGpgRequest {
    pub user_id: i64,
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
