use callisto::gpg_key;
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

impl GpgKey {
    pub fn from_stored(user_id: String, key: gpg_key::Model) -> Self {
        Self {
            user_id,
            key_id: key.key_id,
            fingerprint: key.fingerprint,
            created_at: key.created_at.and_utc(),
            expires_at: key.expires_at.map(|dt| dt.and_utc()),
        }
    }
}
