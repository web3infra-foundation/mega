use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use async_trait::async_trait;

use jupiter::context::Context;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::tree::TreeItem;

use crate::api_service::{ApiHandler, GitObjectCache};
use crate::model::git::CreateFileInfo;
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

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        let storage = self.context.services.git_db_storage.clone();
        let commit = storage
            .get_commit_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap();
        commit.map(|x| x.into())
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

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.context.services.git_db_storage.clone();
        let commits = storage
            .get_commits_by_hashes(self.repo.repo_id, &c_hashes)
            .await
            .unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
        let mut cache = GitObjectCache::default();
        let root_commit = self.get_root_commit().await;
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let mut result: HashMap<TreeItem, Option<Commit>> = HashMap::new();
                for item in tree.tree_items {
                    let commit = self
                        .traverse_commit_history(&path, &root_commit, &item, &mut cache)
                        .await;
                    result.insert(item, Some(commit));
                }
                Ok(result)
            }
            None => Ok(HashMap::new()),
        }
    }
}

impl ImportApiService {
    async fn traverse_commit_history(
        &self,
        path: &Path,
        start_commit: &Commit,
        target: &TreeItem,
        cache: &mut GitObjectCache,
    ) -> Commit {
        let mut target_commit = start_commit.clone();
        let mut visited = HashSet::new();
        let mut p_stack = VecDeque::new();

        visited.insert(start_commit.id);
        p_stack.push_back(start_commit.clone());

        while let Some(commit) = p_stack.pop_front() {
            let root_tree = self.get_tree_from_cache(commit.tree_id, cache).await;
            let reachable = self
                .reachable_in_tree(&root_tree, path, target, cache)
                .await
                .unwrap();
            if reachable {
                for p_id in commit.parent_commit_ids.clone() {
                    if !visited.contains(&p_id) {
                        let p_commit = self.get_commit_from_cache(p_id, cache).await.unwrap();
                        p_stack.push_back(p_commit);
                        visited.insert(p_id);
                    }
                }
                if target_commit.committer.timestamp > commit.committer.timestamp {
                    target_commit = commit.clone();
                }
            }
        }
        target_commit
    }

    async fn get_tree_from_cache(&self, oid: SHA1, cache: &mut GitObjectCache) -> Tree {
        if let Some(tree) = cache.trees.get(&oid) {
            return tree.clone();
        }
        let tree = self.get_tree_by_hash(&oid.to_string()).await;
        cache.trees.insert(oid, tree.clone());
        tree
    }

    async fn get_commit_from_cache(
        &self,
        oid: SHA1,
        cache: &mut GitObjectCache,
    ) -> Result<Commit, GitError> {
        if let Some(commit) = cache.commits.get(&oid) {
            return Ok(commit.clone());
        }
        match self.get_commit_by_hash(&oid.to_string()).await {
            Some(c) => {
                cache.commits.insert(oid, c.clone());
                Ok(c)
            }
            None => Err(GitError::InvalidCommitObject),
        }
    }

    async fn reachable_in_tree(
        &self,
        root_tree: &Tree,
        path: &Path,
        target: &TreeItem,
        cache: &mut GitObjectCache,
    ) -> Result<bool, GitError> {
        let relative_path = self.strip_relative(path).unwrap();
        let mut search_tree = root_tree.clone();
        // first find search tree by path
        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);
                if let Some(search_res) = search_res {
                    search_tree = self.get_tree_from_cache(search_res.id, cache).await;
                } else {
                    return Ok(false);
                }
            }
        }
        // check item exist under search tree
        if search_tree.tree_items.iter().any(|x| x == target) {
            return Ok(true);
        }
        Ok(false)
    }
}
