use std::collections::HashMap;

use std::sync::Arc;

use axum::body::Full;
use axum::response::{IntoResponse, Json};
use axum::{http::StatusCode, response::Response};

use database::driver::ObjectStorage;
use git::internal::object::commit::Commit;
use git::internal::object::tree::Tree;
use git::internal::object::ObjectT;
use hyper::body::Bytes;

use crate::model::object_detail::{BlobObjects, Directories, Item};
use crate::model::query::DirectoryQuery;

pub struct ObjectService {
    pub storage: Arc<dyn ObjectStorage>,
}

const SIGNATURE_END: &str = "-----END PGP SIGNATURE-----";

impl ObjectService {
    pub async fn get_blob_objects(
        &self,
        object_id: &str,
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

    pub async fn get_directories(
        &self,
        query: DirectoryQuery,
    ) -> Result<Json<Directories>, (StatusCode, String)> {
        let DirectoryQuery {
            object_id,
            repo_path,
        } = query;
        if let Some(obj_id) = object_id {
            self.get_tree_objects(&obj_id).await
        } else {
            let directory = self
                .storage
                .get_directory_by_full_path(&repo_path)
                .await
                .unwrap();
            match directory {
                Some(dir) => {
                    if dir.is_repo {
                        // find commit by path
                        let commit_id = match self.storage.search_refs(&repo_path).await {
                            Ok(refs) if !refs.is_empty() => refs[0].ref_git_id.clone(),
                            _ => {
                                return Err((
                                    StatusCode::NOT_FOUND,
                                    "repo_path might not valid".to_string(),
                                ))
                            }
                        };
                        // find tree by commit
                        let tree_id = match self.storage.get_commit_by_hash(&commit_id).await {
                            Ok(Some(commit)) => commit.tree,
                            _ => return Err((StatusCode::NOT_FOUND, "Tree not found".to_string())),
                        };
                        self.get_tree_objects(&tree_id).await
                    } else {
                        let dirs = self.storage.get_directory_by_pid(dir.id).await.unwrap();
                        let items = dirs.into_iter().map(|x| x.into()).collect();
                        let data = Directories { items };
                        Ok(Json(data))
                    }
                }
                None => Err((
                    StatusCode::NOT_FOUND,
                    "repo_path might not valid".to_string(),
                )),
            }
        }
    }

    pub async fn get_tree_objects(
        &self,
        object_id: &str,
    ) -> Result<Json<Directories>, (StatusCode, String)> {
        let tree_data = match self.storage.get_obj_data_by_id(object_id).await {
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
        let child_ids = tree
            .tree_items
            .iter()
            .map(|tree_item| tree_item.id.to_plain_str())
            .collect();

        let child_nodes = self.storage.get_nodes_by_hashes(child_ids).await.unwrap();

        let mut items: Vec<Item> = child_nodes
            .iter()
            .map(|node| Item::from(node.clone()))
            .collect();
        let related_commit_ids = child_nodes.into_iter().map(|x| x.last_commit).collect();
        let related_c = self
            .storage
            .get_commit_by_hashes(related_commit_ids)
            .await
            .unwrap();
        let mut related_c_map: HashMap<String, Commit> = HashMap::new();
        for c in related_c {
            related_c_map.insert(c.git_id.clone(), c.into());
        }

        for item in &mut items {
            let related_c_id = item.commit_id.clone().unwrap();
            let commit = related_c_map.get(&related_c_id).unwrap();
            item.commit_msg = Some(remove_useless_str(
                commit.message.clone(),
                SIGNATURE_END.to_owned(),
            ));
            item.commit_date = Some(commit.committer.timestamp.to_string());
        }

        let data = Directories { items };
        Ok(Json(data))
    }

    pub async fn get_objects_data(
        &self,
        object_id: &str,
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

fn remove_useless_str(content: String, remove_str: String) -> String {
    if let Some(index) = content.find(&remove_str) {
        let filtered_text = &content[index + remove_str.len()..].replace('\n', "");
        let truncated_text = filtered_text.chars().take(50).collect::<String>();
        truncated_text.to_owned()
    } else {
        "".to_owned()
    }
}
