use clap::Args;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Args, Clone, Debug)]
pub struct CommonOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
}

#[derive(Args, Clone, Debug, Default)]
pub struct P2pOptions {
    #[arg(long)]
    pub bootstrap_node: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct InfoRefsParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
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

#[derive(Deserialize)]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Pagination {
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Deserialize)]
pub struct PageParams<T> {
    pub pagination: Pagination,
    pub additional: T,
}

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]

pub struct CommonPage<T> {
    pub total: u64,
    pub items: Vec<T>,
}
