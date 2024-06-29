use serde::Deserialize;
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DirectoryQuery {
    #[serde(default)] // Use default value if not provided in the query string
    pub object_id: Option<String>,
    #[serde(default = "default_path")]
    pub repo_path: String,
}

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
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    "/".to_string()
}
