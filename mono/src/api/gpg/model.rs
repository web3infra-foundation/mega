use utoipa::ToSchema;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewGpgRequest {
    pub user_id: i64,
    pub gpg_content: String,
    pub expires_days: Option<i32>,
}