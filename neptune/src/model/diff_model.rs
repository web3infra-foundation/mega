use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct DiffItem {
    pub path: String,
    pub data: String,
}
