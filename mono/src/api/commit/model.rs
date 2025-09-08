use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitBinding {
    pub id: String,
    pub commit_sha: String,
    pub author_email: String,
    pub matched_user_id: Option<String>,
    pub is_anonymous: bool,
    pub matched_at: Option<String>,
    pub created_at: String,
    pub user: Option<UserInfo>, // Enhanced: Include user object information
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitBindingResponse {
    pub binding: Option<CommitBinding>,
    pub display_name: String, // Enhanced: Ready-to-use display name
    pub avatar_url: Option<String>, // Enhanced: Ready-to-use avatar URL
    pub is_verified_user: bool, // Enhanced: Whether the user is verified in the system
}
