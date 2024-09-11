use serde::{Deserialize, Serialize};

use callisto::{mega_mr, mega_mr_conv};

#[derive(Serialize, Deserialize)]
pub struct MrInfoItem {
    pub mr_link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub updated_at: i64,
}

impl From<mega_mr::Model> for MrInfoItem {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            mr_link: value.mr_link,
            title: String::new(),
            status: value.status.to_string(),
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MRDetail {
    pub id: i64,
    pub mr_link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub conversions: Vec<MRConversion>,
}

#[derive(Serialize, Deserialize)]
pub struct MRConversion {
    pub user_id: i64,
    pub conv_type: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<mega_mr::Model> for MRDetail {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            id: value.id,
            mr_link: value.mr_link,
            title: String::new(),
            status: value.status.to_string(),
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            conversions: vec![],
        }
    }
}

impl From<mega_mr_conv::Model> for MRConversion {
    fn from(value: mega_mr_conv::Model) -> Self {
        Self {
            user_id: value.user_id,
            conv_type: value.conv_type.to_string(),
            created_at: value.created_at.and_utc().timestamp(),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}
