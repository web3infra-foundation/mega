use std::str::FromStr;

use chrono::NaiveDateTime;

use callisto::{db_enums::MergeStatus, mega_mr};
use common::utils::generate_id;

use crate::hash::SHA1;

#[derive(Clone)]
pub struct MergeRequest {
    pub id: i64,
    pub mr_link: String,
    pub status: MergeStatus,
    pub message: Option<String>,
    pub merge_date: Option<NaiveDateTime>,
    pub path: String,
    pub commit_hash: SHA1,
}

impl Default for MergeRequest {
    fn default() -> Self {
        Self {
            id: generate_id(),
            mr_link: String::new(),
            status: MergeStatus::Open,
            message: None,
            merge_date: None,
            path: String::new(),
            commit_hash: SHA1::default(),
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
            path: value.path,
            commit_hash: value.commit_hash.to_plain_str(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<mega_mr::Model> for MergeRequest {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            id: value.id,
            mr_link: String::new(),
            status: value.status,
            merge_date: value.merge_date,
            message: value.mr_msg,
            path: value.path,
            commit_hash: SHA1::from_str(&value.commit_hash).unwrap(),
        }
    }
}
