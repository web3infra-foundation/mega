use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use async_trait::async_trait;

use jupiter::context::Context;
use mercury::errors::GitError;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::tree::TreeItem;

use crate::api_service::ApiHandler;
use crate::model::create_file::CreateFileInfo;
use crate::protocol::repo::Repo;

#[derive(Clone)]
pub struct ImportApiService {
    pub context: Context,
    pub repo: Repo,
}

#[async_trait]
impl ApiHandler for ImportApiService {
    fn get_context(&self) -> Context {
        self.context.clone()
    }

    async fn create_monorepo_file(&self, _: CreateFileInfo) -> Result<(), GitError> {
        return Err(GitError::CustomError(
            "import dir does not support create file".to_string(),
        ));
    }


    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError> {
        if let Ok(relative_path) = path.strip_prefix(self.repo.repo_path.clone()) {
            Ok(relative_path.to_path_buf())
        } else {
            Err(GitError::CustomError(
                "The full path does not start with the base path.".to_string(),
            ))
        }
    }

    async fn get_root_commit(&self) -> Commit {
        let storage = self.context.services.git_db_storage.clone();
        let refs = storage
            .get_default_ref(self.repo.repo_id)
            .await
            .unwrap()
            .unwrap();
        storage
            .get_commit_by_hash(self.repo.repo_id, &refs.ref_git_id)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_root_tree(&self) -> Tree {
        let storage = self.context.services.git_db_storage.clone();
        let refs = storage
            .get_default_ref(self.repo.repo_id)
            .await
            .unwrap()
            .unwrap();

        let root_commit = storage
            .get_commit_by_hash(self.repo.repo_id, &refs.ref_git_id)
            .await
            .unwrap()
            .unwrap();
        storage
            .get_tree_by_hash(self.repo.repo_id, &root_commit.tree)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        self.context
            .services
            .git_db_storage
            .get_tree_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit {
        let storage = self.context.services.git_db_storage.clone();
        let tree_info = storage
            .get_tree_by_hash(self.repo.repo_id, t_hash)
            .await
            .unwrap()
            .unwrap();
        storage
            .get_commit_by_hash(self.repo.repo_id, &tree_info.commit_id)
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
            .get_trees_by_hashes(self.repo.repo_id, hashes)
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
            .get_blobs_by_hashes(self.repo.repo_id, hashes)
            .await
            .unwrap();
        for blob in blobs {
            item_to_commit.insert(blob.blob_id, blob.commit_id);
        }
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.context.services.git_db_storage.clone();
        let commits = storage
            .get_commits_by_hashes(self.repo.repo_id, &c_hashes)
            .await
            .unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
    }

    async fn traverse_commit_history(
        &self,
        path: &Path,
        start_commit: Commit,
        target: &TreeItem,
    ) -> Commit {
        let mut target_commit = start_commit.clone();
        let mut visited = HashSet::new();
        let mut p_stack = VecDeque::new();

        visited.insert(start_commit.id);
        p_stack.push_back(start_commit);

        while let Some(commit) = p_stack.pop_front() {
            let root_tree = self.get_tree_by_hash(&commit.tree_id.to_string()).await;
            let reachable = self
                .reachable_in_tree(&root_tree, path, target)
                .await
                .unwrap();
            if reachable {
                let mut p_ids = vec![];
                for p_id in commit.parent_commit_ids.clone() {
                    if !visited.contains(&p_id) {
                        p_ids.push(p_id.to_string());
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
