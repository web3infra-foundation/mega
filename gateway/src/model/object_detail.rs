use serde::{Deserialize, Serialize};

use entity::{node, repo_directory};

#[derive(Serialize, Deserialize)]
pub struct Directories {
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub path: String,
    pub content_type: String,
    pub under_repo: bool,
    pub commit_msg: Option<String>,
    pub commit_date: Option<String>,
    pub commit_id: Option<String>,
}

impl From<node::Model> for Item {
    fn from(val: node::Model) -> Self {
        let content_type = match val.node_type.as_str() {
            "blob" => "file".to_owned(),
            "tree" => "directory".to_owned(),
            _ => unreachable!("not supported type"),
        };
        Item {
            id: val.git_id,
            name: val.name.unwrap(),
            path: val.full_path,
            content_type,
            under_repo: true,
            commit_msg: None,
            commit_date: None,
            commit_id: Some(val.last_commit),
        }
    }
}

impl From<repo_directory::Model> for Item {
    fn from(value: repo_directory::Model) -> Self {
        Item {
            id: value.id.to_string(),
            name: value.name,
            path: value.full_path,
            content_type: "directory".to_owned(),
            under_repo: value.is_repo,
            commit_msg: None,
            commit_date: None,
            commit_id: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlobObjects {
    pub row_data: String,
}
