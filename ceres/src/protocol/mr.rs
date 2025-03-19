use chrono::NaiveDateTime;

use callisto::{mega_mr, sea_orm_active_enums::MergeStatusEnum};
use common::utils::generate_id;

#[derive(Clone)]
pub struct MergeRequest {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: MergeStatusEnum,
    pub merge_date: Option<NaiveDateTime>,
    pub path: String,
    pub from_hash: String,
    pub to_hash: String,
}

impl Default for MergeRequest {
    fn default() -> Self {
        Self {
            id: generate_id(),
            link: String::new(),
            title: String::new(),
            status: MergeStatusEnum::Open,
            merge_date: None,
            path: String::new(),
            from_hash: String::new(),
            to_hash: String::new(),
        }
    }
}

impl MergeRequest {
    pub fn close(&mut self) {
        self.status = MergeStatusEnum::Closed;
    }

    pub fn merge(&mut self) {
        self.status = MergeStatusEnum::Merged;
        self.merge_date = Some(chrono::Utc::now().naive_utc())
    }
}

impl From<MergeRequest> for mega_mr::Model {
    fn from(value: MergeRequest) -> Self {
        Self {
            id: value.id,
            link: value.link,
            title: value.title,
            status: value.status,
            merge_date: value.merge_date,
            path: value.path,
            from_hash: value.from_hash,
            to_hash: value.to_hash,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<mega_mr::Model> for MergeRequest {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            id: value.id,
            link: value.link,
            title: value.title,
            status: value.status,
            merge_date: value.merge_date,
            path: value.path,
            from_hash: value.from_hash,
            to_hash: value.to_hash,
        }
    }
}
