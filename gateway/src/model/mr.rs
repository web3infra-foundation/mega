use serde::{Deserialize, Serialize};


#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeOperation {
    pub message: Option<String>,
    pub mr_id: i64,
}

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]

pub struct MergeResult {
    pub result: bool,
    pub err_message: String,
}