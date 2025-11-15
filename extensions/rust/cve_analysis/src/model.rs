use serde::{Deserialize, Serialize};
#[derive(Clone,Debug, Default, Serialize, Deserialize)]
pub struct CveId{
    pub id:String,
    pub url:String,
}
#[derive(Clone,Debug, Default, Serialize, Deserialize)]
pub struct CveAnalyzeRes{
    pub crate_version:String,
    pub dept_crate_version:Vec<String>,
}