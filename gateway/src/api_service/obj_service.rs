use std::sync::Arc;

use axum::body::Full;
use axum::response::{IntoResponse, Json};
use axum::{http::StatusCode, response::Response};

use database::driver::ObjectStorage;
use git::internal::object::tree::Tree;
use git::internal::object::ObjectT;
use hyper::body::Bytes;

use crate::model::object_detail::{BlobObjects, Item, TreeObjects};

pub struct ObjectService {
    pub storage: Arc<dyn ObjectStorage>,
}

impl ObjectService {
    pub async fn get_blob_objects(
        &self,
        object_id: &str,
        _repo_path: &str,
    ) -> Result<Json<BlobObjects>, (StatusCode, String)> {
        let blob_data = match self.storage.get_obj_data_by_id(object_id).await {
            Ok(Some(node)) => {
                if node.object_type == "blob" {
                    node.data
                } else {
                    return Err((StatusCode::NOT_FOUND, "Blob not found".to_string()));
                }
            }
            _ => return Err((StatusCode::NOT_FOUND, "Blob not found".to_string())),
        };

        let row_data = match String::from_utf8(blob_data) {
            Ok(str) => str,
            _ => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Can not convert blob to readable txt".to_string(),
                ))
            }
        };

        let data = BlobObjects { row_data };
        Ok(Json(data))
    }

    pub async fn get_tree_objects(
        &self,
        object_id: Option<&String>,
        repo_path: &str,
    ) -> Result<Json<TreeObjects>, (StatusCode, String)> {
        let tree_id = if let Some(object_id) = object_id {
            object_id.to_owned()
        } else {
            let commit_id = match self.storage.search_refs(repo_path).await {
                Ok(refs) if !refs.is_empty() => refs[0].ref_git_id.clone(),
                _ => {
                    return Err((
                        StatusCode::NOT_FOUND,
                        "repo_path might not valid".to_string(),
                    ))
                }
            };
            match self.storage.get_commit_by_hash(&commit_id).await {
                Ok(Some(commit)) => commit.tree,
                _ => return Err((StatusCode::NOT_FOUND, "Tree not found".to_string())),
            }
        };

        let tree_data = match self.storage.get_obj_data_by_id(&tree_id).await {
            Ok(Some(node)) => {
                if node.object_type == "tree" {
                    node.data
                } else {
                    return Err((StatusCode::NOT_FOUND, "Tree not found".to_string()));
                }
            }
            _ => return Err((StatusCode::NOT_FOUND, "Tree not found".to_string())),
        };

        let tree = Tree::new_from_data(tree_data);
        let items = tree
            .tree_items
            .iter()
            .map(|tree_item| Item::from(tree_item.clone()))
            .collect();

        let data = TreeObjects { items };
        Ok(Json(data))
    }

    pub async fn get_objects_data(
        &self,
        object_id: &str,
        _repo_path: &str,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let node = match self.storage.get_node_by_hash(object_id).await {
            Ok(Some(node)) => node,
            _ => return Err((StatusCode::NOT_FOUND, "Blob not found".to_string())),
        };
        let raw_data = match self.storage.get_obj_data_by_id(object_id).await {
            Ok(Some(model)) => model,
            _ => return Err((StatusCode::NOT_FOUND, "Blob not found".to_string())),
        };
        let body = Full::new(Bytes::from(raw_data.data));

        let file_name = format!("inline; filename=\"{}\"", node.name.unwrap());
        let res = Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", file_name)
            .body(body)
            .unwrap();
        Ok(res)
    }
}
