use std::borrow::BorrowMut;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::path::{self, PathBuf};
use std::sync::Arc;

use async_recursion::async_recursion;
use axum::body::Full;
use axum::response::{IntoResponse, Json};
use axum::{http::StatusCode, response::Response};

use database::driver::ObjectStorage;
use entity::commit;
use git::internal::object::tree::Tree;
use git::internal::object::ObjectT;
use hyper::body::Bytes;

use crate::model::object_detail::{BlobObjects, Item, TreeObjects};

pub struct ObjectService {
    pub storage: Arc<dyn ObjectStorage>,
}

pub struct CommitSearcher {
    pub storage: Arc<dyn ObjectStorage>,
    pub search_cache: HashMap<String, Tree>,
}

const SIGNATURE_END: &str = "-----END PGP SIGNATURE-----";
const COMMITTER_END: &str = "Date: ";

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

        let commits: Vec<entity::commit::Model> = self
            .storage
            .get_all_commits_by_path(repo_path)
            .await
            .unwrap();

        let mut commit_searcher = CommitSearcher {
            storage: self.storage.clone(),
            search_cache: HashMap::new(),
        };

        // build graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for c in &commits {
            let mut path = c.pid.clone();
            path.reverse();
            graph.insert(c.git_id.clone(), path);
        }
        let root_commit = self
            .storage
            .get_ref_object_id(repo_path)
            .await
            .unwrap()
            .iter()
            .find(|m| m.ref_name == "refs/heads/main")
            .cloned()
            .expect("can't find main ref");
        let visited = bfs(&graph, root_commit.ref_git_id);
        let commit_map: HashMap<String, commit::Model> =
            commits.into_iter().map(|c| (c.git_id.clone(), c)).collect();

        for item in &mut items {
            let target_id = &item.id;
            let relative_path = &item.path.replace(repo_path, "");
            for c in &visited {
                let commit = commit_map.get(c).unwrap();
                // skip merge commits
                if commit.pid.len() <= 1 {
                    let tree_id = commit.tree.clone();
                    let search_res = commit_searcher
                        .search(target_id, tree_id, PathBuf::from(relative_path))
                        .await;
                    if search_res {
                        item.commit_id = Some(commit.git_id.clone());
                        item.commit_msg = Some(remove_useless_str(
                            commit.content.clone().unwrap(),
                            SIGNATURE_END.to_owned(),
                        ));
                        item.commit_date = Some(remove_useless_str(
                            commit.committer.clone().unwrap(),
                            COMMITTER_END.to_owned(),
                        ));
                        break;
                    }
                }
            }
        }

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

impl CommitSearcher {
    #[async_recursion]
    pub async fn search(
        &mut self,
        target_id: &str,
        tree_id: String,
        relative_path: PathBuf,
    ) -> bool {
        let search_cache = self.search_cache.borrow_mut();

        let tree = match search_cache.get(&tree_id) {
            Some(tree) => tree.to_owned(),
            None => {
                let model = self.storage.get_obj_data_by_id(&tree_id).await.unwrap();
                let tree = Tree::new_from_data(model.unwrap().data);
                search_cache.insert(tree_id, tree.clone());
                tree
            }
        };

        let mut path_iter = relative_path.iter();
        let mut root_path = path_iter.next();
        if root_path.eq(&Some(OsStr::new(&path::MAIN_SEPARATOR.to_string()))) {
            root_path = path_iter.next()
        };

        match root_path {
            Some(parent) => {
                for t_item in &tree.tree_items {
                    // find the directory with same name
                    if OsStr::new(&t_item.name).eq(parent) {
                        if path_iter.clone().next().is_some() {
                            return self
                                .search(
                                    target_id,
                                    t_item.id.to_plain_str(),
                                    path_iter.clone().collect(),
                                )
                                .await;
                        } else if !t_item.id.to_plain_str().eq(&target_id) {
                            return true;
                        }
                    }
                }
            }
            None => {
                panic!("can't parse path")
            }
        }
        false
    }
}

fn bfs(graph: &HashMap<String, Vec<String>>, start: String) -> Vec<String> {
    let mut visited = Vec::new();
    let mut queue = VecDeque::new();

    queue.push_back(start.clone());
    visited.push(start.clone());

    while let Some(node) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    queue.push_back(neighbor.clone());
                    visited.push(neighbor.clone());
                }
            }
        }
    }
    visited
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
