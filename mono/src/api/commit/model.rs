use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitBindingResponse {
    pub display_name: String,       // Ready-to-use display name
    pub avatar_url: Option<String>, // Ready-to-use avatar URL
    pub is_verified_user: bool,     // Whether the user is verified in the system
}
