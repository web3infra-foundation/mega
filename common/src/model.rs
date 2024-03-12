use clap::Args;
use serde::Deserialize;

use crate::enums::DataSource;


#[derive(Args, Clone, Debug)]
pub struct CommonOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
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
