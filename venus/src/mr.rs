use callisto::{db_enums::MergeStatus, mega_mr};
use chrono::NaiveDateTime;
use common::utils::generate_id;

#[derive(Clone)]
pub struct MergeRequest {
    pub id: i64,
    pub mr_link: String,
    pub status: MergeStatus,
    pub message: Option<String>,
    pub merge_date: Option<NaiveDateTime>,
}

impl Default for MergeRequest {
    fn default() -> Self {
        Self {
            id: generate_id(),
            mr_link: String::new(),
            status: MergeStatus::Open,
            message: None,
            merge_date: None,
        }
    }
}

impl MergeRequest {
    pub fn close(&mut self, msg: Option<String>) {
        self.status = MergeStatus::Closed;
        self.message = msg;
    }

    pub fn merge(&mut self, msg: Option<String>) {
        self.status = MergeStatus::Merged;
        self.message = msg;
        self.merge_date = Some(chrono::Utc::now().naive_utc())
    }
}

impl From<MergeRequest> for mega_mr::Model {
    fn from(value: MergeRequest) -> Self {
        Self {
            id: value.id,
            mr_link: String::new(),
            status: value.status,
            merge_date: value.merge_date,
            mr_msg: value.message,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
