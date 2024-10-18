use serde::Deserialize;

#[derive(Deserialize)]
pub struct MRStatusParams {
    pub status: String,
}

