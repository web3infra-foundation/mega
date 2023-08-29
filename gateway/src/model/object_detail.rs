use entity::node;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TreeObjects {
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub path: String,
    pub content_type: String,
    pub mr_msg: Option<String>,
    pub mr_date: Option<String>,
    pub mr_id: Option<i64>,
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
            mr_msg: None,
            mr_date: None,
            mr_id: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlobObjects {
    pub row_data: String,
}
