use clap::Args;
use serde::Deserialize;


#[derive(Args, Clone, Debug)]
pub struct CommonOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
}


#[derive(Deserialize, Debug)]
pub struct GetParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
    pub id: Option<String>,
    pub path: Option<String>,
    pub limit: Option<String>,
    pub cursor: Option<String>,
}
