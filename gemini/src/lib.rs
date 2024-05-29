use serde::Deserialize;

pub mod http;
pub mod ztm;

#[derive(Deserialize, Debug)]
pub struct RelayGetParams {
    pub name: Option<String>,
}
