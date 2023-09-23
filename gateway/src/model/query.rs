use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DirectoryQuery {
    #[serde(default)] // Use default value if not provided in the query string
    pub object_id: Option<String>,
    #[serde(default = "default_path")]
    pub repo_path: String,
}

fn default_path() -> String {
    "/root".to_string()
}
