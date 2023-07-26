use git::internal::object::tree::{TreeItem, TreeItemMode};
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
}

impl From<TreeItem> for Item {
    fn from(val: TreeItem) -> Self {
        let content_type = match val.mode {
            TreeItemMode::Blob => "file".to_owned(),
            TreeItemMode::Tree => "directory".to_owned(),
            _ => unreachable!("not supported type"),
        };
        Item {
            id: val.id.to_plain_str(),
            name: val.name.clone(),
            path: "a".to_owned() + &val.name,
            content_type,
        }
    }
}


#[derive(Serialize, Deserialize)]
pub struct BlobObjects {
    pub row_data: String,
}