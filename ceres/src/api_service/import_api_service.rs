use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use jupiter::storage::Storage;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::tree::TreeItem;
use mercury::internal::object::tree::TreeItemMode;
use tokio::sync::Mutex;

use crate::api_service::{ApiHandler, GitObjectCache};
use crate::model::git::CreateFileInfo;
use crate::protocol::repo::Repo;

#[derive(Clone)]
pub struct ImportApiService {
    pub storage: Storage,
    pub repo: Repo,
}

#[async_trait]
impl ApiHandler for ImportApiService {
    fn get_context(&self) -> Storage {
        self.storage.clone()
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
        let storage = self.storage.git_db_storage();
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
        let storage = self.storage.git_db_storage();
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
        self.storage
            .git_db_storage()
            .get_tree_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        let storage = self.storage.git_db_storage();
        let commit = storage
            .get_commit_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap();
        commit.map(|x| x.into())
    }

    async fn get_tree_relate_commit(
        &self,
        t_hash: SHA1,
        path: PathBuf,
    ) -> Result<Commit, GitError> {
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => {
                return Err(GitError::CustomError("Invalid Path Input".to_string()));
            }
        };

        let search_item = TreeItem::new(TreeItemMode::Tree, t_hash, file_name);
        let cache = GitObjectCache::new();
        let root_commit = Arc::new(self.get_root_commit().await);

        let parent = match path.parent() {
            Some(p) => p,
            None => return Err(GitError::CustomError("Invalid Path Input".to_string())),
        };
        self.traverse_commit_history(parent, root_commit, &search_item, cache)
            .await
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.storage.git_db_storage();
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
        let cache = GitObjectCache::new();
        let root_commit = Arc::new(self.get_root_commit().await);
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let mut result: HashMap<TreeItem, Option<Commit>> = HashMap::new();
                for item in tree.tree_items {
                    let commit = self
                        .traverse_commit_history(&path, root_commit.clone(), &item, cache.clone())
                        .await?;
                    result.insert(item, Some(commit));
                }
                Ok(result)
            }
            None => Ok(HashMap::new()),
        }
    }
}

impl ImportApiService {
    /// Traverses the commit history starting from a given commit, looking for the earliest commit
    /// (based on committer timestamp) where the target `TreeItem` is reachable at the given path.
    ///
    /// The function performs a breadth-first search (BFS) through the commit graph, checking for the
    /// target's existence in each commit's tree. It uses a commit and tree cache to avoid redundant
    /// repository lookups.
    ///
    /// # Arguments
    /// * `path_components` - The path to search, pre-split into components to avoid repeated parsing.
    /// * `start_commit` - The commit to start traversal from.
    /// * `target` - The tree item we want to find under path (e.g., file or subdirectory).
    /// * `cache` - A shared, mutable cache of commits and trees to speed up lookups.
    ///
    /// # Returns
    /// The earliest commit (by timestamp) in which the target path contains the given `TreeItem`.
    ///
    /// # Algorithm
    /// 1. Initialize a queue with the starting commit.
    /// 2. Track visited commit IDs to prevent cycles.
    /// 3. For each commit in the queue:
    ///     - Load its root tree from the cache (or repository if not cached).
    ///     - Check if the `target` is reachable at the given path.
    ///     - If reachable:
    ///         - Add unvisited parent commits to the queue.
    ///         - If this commit has an earlier timestamp than the current best match, update the result.
    /// 4. Return the earliest matching commit.
    ///
    /// # Performance Notes
    /// - Uses `Arc<Commit>` internally to avoid cloning commits during traversal.
    /// - Commit and tree lookups are cached in `GitObjectCache`.
    ///
    /// # Locking
    /// - `cache` is wrapped in `Arc<Mutex<_>>` for safe concurrent access across async calls.
    async fn traverse_commit_history(
        &self,
        path: &Path,
        start_commit: Arc<Commit>,
        search_item: &TreeItem,
        cache: Arc<Mutex<GitObjectCache>>,
    ) -> Result<Commit, GitError> {
        let mut target_commit = start_commit.clone();
        let mut visited = HashSet::new();
        let mut p_stack = VecDeque::new();

        visited.insert(start_commit.id);
        p_stack.push_back(start_commit);

        while let Some(commit) = p_stack.pop_front() {
            let root_tree = {
                let mut cache_lock = cache.lock().await;
                self.get_tree_from_cache(commit.tree_id, &mut cache_lock)
                    .await?
            };

            let reachable = {
                let mut cache_lock = cache.lock().await;
                self.reachable_in_tree(root_tree, path, search_item, &mut cache_lock)
                    .await?
            };

            if reachable {
                for &p_id in &commit.parent_commit_ids {
                    if !visited.contains(&p_id) {
                        let p_commit = {
                            let mut cache_lock = cache.lock().await;
                            self.get_commit_from_cache(p_id, &mut cache_lock).await?
                        };
                        p_stack.push_back(p_commit.clone());
                        visited.insert(p_id);
                    }
                }
                if target_commit.committer.timestamp > commit.committer.timestamp {
                    target_commit = commit.clone();
                }
            }
        }
        Ok((*target_commit).clone())
    }

    async fn get_tree_from_cache(
        &self,
        oid: SHA1,
        cache: &mut GitObjectCache,
    ) -> Result<Arc<Tree>, GitError> {
        if let Some(tree) = cache.trees.get(&oid) {
            return Ok(tree.clone());
        }
        let tree = Arc::new(self.get_tree_by_hash(&oid.to_string()).await);
        cache.trees.insert(oid, tree.clone());
        Ok(tree)
    }

    async fn get_commit_from_cache(
        &self,
        oid: SHA1,
        cache: &mut GitObjectCache,
    ) -> Result<Arc<Commit>, GitError> {
        if let Some(commit) = cache.commits.get(&oid) {
            return Ok(commit.clone());
        }
        match self.get_commit_by_hash(&oid.to_string()).await {
            Some(c) => {
                let commit = Arc::new(c);
                cache.commits.insert(oid, commit.clone());
                Ok(commit)
            }
            None => Err(GitError::InvalidCommitObject),
        }
    }

    async fn reachable_in_tree(
        &self,
        root_tree: Arc<Tree>,
        path: &Path,
        search_item: &TreeItem,
        cache: &mut GitObjectCache,
    ) -> Result<bool, GitError> {
        let relative_path = self.strip_relative(path)?;
        let mut search_tree = root_tree;
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
                    search_tree = self.get_tree_from_cache(search_res.id, cache).await?;
                } else {
                    return Ok(false);
                }
            }
        }
        // check item exist under search tree
        if search_tree.tree_items.iter().any(|x| x == search_item) {
            return Ok(true);
        }
        Ok(false)
    }
}
