use serde::Deserialize;
use utoipa::ToSchema;

use jupiter::storage::stg_common::model::ListParams;

#[derive(Deserialize, ToSchema)]
pub struct LabelUpdatePayload {
    pub label_ids: Vec<i64>,
    pub item_id: i64,
    pub link: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AssigneeUpdatePayload {
    pub assignees: Vec<String>,
    pub item_id: i64,
    pub link: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ListPayload {
    pub status: String,
    pub author: Option<String>,
    pub labels: Option<Vec<i64>>,
    pub assignees: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub asc: bool,
}

impl From<ListPayload> for ListParams {
    fn from(value: ListPayload) -> Self {
        Self {
            status: value.status,
            author: value.author,
            labels: value.labels,
            assignees: value.assignees,
            sort_by: value.sort_by,
            asc: value.asc,
        }
    }
}
