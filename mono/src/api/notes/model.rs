use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRequest {
    pub description_html: String,
    pub description_state: String,
    pub description_schema_version: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ShowResponse {
    #[serde(rename = "id")]
    pub public_id: String,

    pub description_schema_version: i32,

    #[serde(rename = "description_state", skip_serializing_if = "Option::is_none")]
    pub description_state: Option<String>,

    pub description_html: String,
}
