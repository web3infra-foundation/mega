use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub diff: String,
    //..more fields
}

#[derive(Debug, Serialize)]
pub struct ReviewSuggestRes {
    pub suggestions: Vec<String>,
    //..more fields
}
