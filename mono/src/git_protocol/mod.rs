use serde::Deserialize;

pub mod http;
pub mod protocol_error;
pub mod ssh;

#[derive(Deserialize, Debug)]
pub struct InfoRefsParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
}
