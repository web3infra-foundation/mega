use callisto::label;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LabelItem {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: String,
}

impl From<label::Model> for LabelItem {
    fn from(value: label::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            color: value.color,
            description: value.description,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewLabel {
    pub name: String,
    pub color: String,
    pub description: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LabelUpdatePayload {
    pub label_ids: Vec<i64>,
    pub item_id: i64,
    pub link: String,
}
