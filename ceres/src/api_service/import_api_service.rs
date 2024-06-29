use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use axum::async_trait;

use callisto::raw_blob;
use common::errors::MegaError;
use jupiter::context::Context;
use mercury::errors::GitError;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::tree::TreeItem;
use venus::import_repo::repo::Repo;

use crate::api_service::ApiHandler;
use crate::model::create_file::CreateFileInfo;

#[derive(Clone)]
pub struct ImportApiService {
    pub context: Context,
    pub repo: Repo,
}

#[async_trait]
impl ApiHandler for ImportApiService {
    async fn create_monorepo_file(&self, _: CreateFileInfo) -> Result<(), GitError> {
        return Err(GitError::CustomError(
            "import dir does not support create file".to_string(),
        ));
    }

    async fn get_raw_blob_by_hash(&self, hash: &str) -> Result<Option<raw_blob::Model>, MegaError> {
        self.context
            .services
            .mega_storage
            .get_raw_blob_by_hash(hash)
            .await
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError> {
        if let Ok(relative_path) = path.strip_prefix(self.repo.repo_path.clone()) {
            Ok(relative_path.to_path_buf())
        } else {
            Err(GitError::ConversionError(
                "The full path does not start with the base path.".to_string(),
            ))
        }
    }
    async fn get_root_commit(&self) -> Commit {
        let storage = self.context.services.git_db_storage.clone();
        let refs = storage.get_default_ref(&self.repo).await.unwrap().unwrap();
        storage
            .get_commit_by_hash(&self.repo, &refs.ref_hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_root_tree(&self) -> Tree {
        let storage = self.context.services.git_db_storage.clone();
        let refs = storage.get_default_ref(&self.repo).await.unwrap().unwrap();

        let root_commit = storage
            .get_commit_by_hash(&self.repo, &refs.ref_hash)
            .await
            .unwrap()
            .unwrap();
        storage
            .get_tree_by_hash(&self.repo, &root_commit.tree)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        self.context
            .services
            .git_db_storage
            .get_tree_by_hash(&self.repo, hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit {
        let storage = self.context.services.git_db_storage.clone();
        let tree_info = storage
            .get_tree_by_hash(&self.repo, t_hash)
            .await
            .unwrap()
            .unwrap();
        storage
            .get_commit_by_hash(&self.repo, &tree_info.commit_id)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn add_trees_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    ) {
        let storage = self.context.services.git_db_storage.clone();
        let trees = storage
            .get_trees_by_hashes(&self.repo, hashes)
            .await
            .unwrap();
        for tree in trees {
            item_to_commit.insert(tree.tree_id, tree.commit_id);
        }
    }

    async fn add_blobs_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    ) {
        let storage = self.context.services.git_db_storage.clone();
        let blobs = storage
            .get_blobs_by_hashes(&self.repo, hashes)
            .await
            .unwrap();
        for blob in blobs {
            item_to_commit.insert(blob.blob_id, blob.commit_id);
        }
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.context.services.git_db_storage.clone();
        let commits = storage
            .get_commits_by_hashes(&self.repo, &c_hashes)
            .await
            .unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
    }

    async fn traverse_commit_history(
        &self,
        path: &Path,
        start_commit: Commit,
        target: TreeItem,
    ) -> Commit {
        let mut target_commit = start_commit.clone();
        let mut visited = HashSet::new();
        let mut p_stack = VecDeque::new();

        visited.insert(start_commit.id);
        p_stack.push_back(start_commit);

        while let Some(commit) = p_stack.pop_front() {
            let root_tree = self.get_tree_by_hash(&commit.tree_id.to_plain_str()).await;
            let reachable = self
                .reachable_in_tree(&root_tree, path, target.clone())
                .await
                .unwrap();
            if reachable {
                let mut p_ids = vec![];
                for p_id in commit.parent_commit_ids.clone() {
                    if !visited.contains(&p_id) {
                        p_ids.push(p_id.to_plain_str());
                        visited.insert(p_id);
                    }
                }
                if target_commit.committer.timestamp > commit.committer.timestamp {
                    target_commit = commit;
                }
                let parent_commits = self.get_commits_by_hashes(p_ids).await.unwrap();
                p_stack.extend(parent_commits);
            }
        }
        target_commit
    }
}

impl ImportApiService {
    // pub async fn get_objects_data(
    //     &self,
    //     object_id: &str,
    //     repo_path: &str,
    // ) -> Result<Response, (StatusCode, String)> {
    //     let node = match self.storage.get_node_by_hash(object_id, repo_path).await {
    //         Ok(Some(node)) => node,
    //         _ => return Err((StatusCode::NOT_FOUND, "Blob not found".to_string())),
    //     };
    //     let raw_data = match self.storage.get_obj_data_by_id(object_id).await {
    //         Ok(Some(model)) => model,
    //         _ => return Err((StatusCode::NOT_FOUND, "Blob not found".to_string())),
    //     };
    //     let file_name = format!("inline; filename=\"{}\"", node.name.unwrap());
    //     let res = Response::builder()
    //         .header("Content-Type", "application/octet-stream")
    //         .header("Content-Disposition", file_name)
    //         .body(raw_data.data.into())
    //         .unwrap();
    //     Ok(res)
    // }

    // pub async fn count_object_num(
    //     &self,
    //     repo_path: &str,
    // ) -> Result<Json<GitTypeCounter>, (StatusCode, String)> {
    //     let query_res = self.storage.count_obj_from_node(repo_path).await.unwrap();
    //     let tree = query_res
    //         .iter()
    //         .find(|x| x.node_type == "tree")
    //         .map(|x| x.count)
    //         .unwrap_or_default()
    //         .try_into()
    //         .unwrap();
    //     let blob = query_res
    //         .iter()
    //         .find(|x| x.node_type == "blob")
    //         .map(|x| x.count)
    //         .unwrap_or_default()
    //         .try_into()
    //         .unwrap();
    //     let commit = self.storage.count_obj_from_commit(repo_path).await.unwrap().try_into().unwrap();
    //     let counter = GitTypeCounter {
    //         commit,
    //         tree,
    //         blob,
    //         tag: 0,
    //         ofs_delta: 0,
    //         ref_delta: 0,
    //     };
    //     Ok(Json(counter))
    // }
}
