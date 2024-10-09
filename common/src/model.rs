use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Clone, Debug)]
pub struct CommonOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
}

#[derive(Args, Clone, Debug)]
pub struct ZtmOptions {
    #[arg(long, default_value_t = 7777)]
    pub ztm_agent_port: u16,

    #[arg(long)]
    pub bootstrap_node: Option<String>,

    #[arg(long, default_value_t = false)]
    pub cache_repo: bool,
}

#[derive(Deserialize, Debug)]
pub struct InfoRefsParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
}

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
