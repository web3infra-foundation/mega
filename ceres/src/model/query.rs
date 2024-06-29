use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CodePreviewQuery {
    #[serde(default)]
    pub refs: String,
    #[serde(default = "default_path")]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct BlobContentQuery {
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    "/".to_string()
}
