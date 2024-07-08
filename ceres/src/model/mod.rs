use serde::{Deserialize, Serialize};

pub mod create_file;
pub mod mr;
pub mod query;
pub mod tree;

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]

pub struct CommonResult<T> {
    pub req_result: bool,
    pub data: Option<T>,
    pub err_message: String,
}

impl<T> CommonResult<T> {
    pub fn success(data: Option<T>) -> Self {
        CommonResult {
            req_result: true,
            data,
            err_message: "".to_owned(),
        }
    }
    pub fn failed(err_message: &str) -> Self {
        CommonResult {
            req_result: false,
            data: None,
            err_message: err_message.to_string(),
        }
    }
}
