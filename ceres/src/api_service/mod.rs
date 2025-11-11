use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;

use callisto::raw_blob;
use common::errors::MegaError;
use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::object::{
        ObjectTrait,
        commit::Commit,
        tree::{Tree, TreeItem, TreeItemMode},
    },
};
use jupiter::{storage::Storage, utils::converter::generate_git_keep_with_timestamp};
use tokio::sync::Mutex;

use crate::model::blame::{BlameQuery, BlameResult};
use crate::model::git::{
    CommitBindingInfo, CreateEntryInfo, DiffPreviewPayload, EditFilePayload, EditFileResult,
    LatestCommitInfo, TreeBriefItem, TreeCommitItem, TreeHashItem,
};
use common::model::{Pagination, TagInfo};

pub mod import_api_service;
pub mod mono_api_service;

#[derive(Debug, Default, Clone)]
pub struct GitObjectCache {
    trees: HashMap<SHA1, Arc<Tree>>,
    commits: HashMap<SHA1, Arc<Commit>>,
}

impl GitObjectCache {
    pub fn new() -> GitObjectCache {
        GitObjectCache::default()
    }
}

// TagInfo moved to `common::model::TagInfo`

#[async_trait]
pub trait ApiHandler: Send + Sync {
    fn get_context(&self) -> Storage;

    /// Create a file or directory entry under the monorepo path. Returns the new commit id on success.
    async fn create_monorepo_entry(&self, file_info: CreateEntryInfo) -> Result<String, GitError>;

    async fn get_raw_blob_by_hash(&self, hash: &str) -> Result<Option<raw_blob::Model>, MegaError> {
        let context = self.get_context();
        context.raw_db_storage().get_raw_blob_by_hash(hash).await
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, MegaError>;

    async fn get_root_tree(&self) -> Tree;

    async fn get_binary_tree_by_path(
        &self,
        path: &Path,
        oid: Option<String>,
    ) -> Result<Vec<u8>, GitError> {
        let Some(tree) = self.search_tree_by_path(path).await.unwrap() else {
            return Ok(vec![]);
        };
        if let Some(oid) = oid
            && oid != tree.id._to_string()
        {
            return Ok(vec![]);
        }
        tree.to_data()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree;

    async fn get_commit_by_hash(&self, hash: &str) -> Commit;

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
        let cache = Arc::new(Mutex::new(GitObjectCache::default()));
        let root_commit = Arc::new(self.get_root_commit().await);

        let parent = match path.parent() {
            Some(p) => p,
            None => return Err(GitError::CustomError("Invalid Path Input".to_string())),
        };
        self.traverse_commit_history(parent, root_commit, &search_item, cache)
            .await
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError>;

    async fn get_blob_as_string(
        &self,
        file_path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Option<String>, GitError> {
        let filename = file_path.file_name().unwrap().to_str().unwrap();
        let parent = file_path.parent().unwrap();
        if let Some(tree) = self.search_tree_by_path_with_refs(parent, refs).await?
            && let Some(item) = tree.tree_items.into_iter().find(|x| x.name == filename)
        {
            match self.get_raw_blob_by_hash(&item.id.to_string()).await {
                Ok(Some(model)) => {
                    return Ok(Some(String::from_utf8(model.data.unwrap()).unwrap()));
                }
                _ => return Ok(None),
            };
        }
        return Ok(None);
    }

    async fn get_latest_commit(&self, path: PathBuf) -> Result<LatestCommitInfo, GitError> {
        // 1) Try as directory path first
        if let Some(tree) = self.search_tree_by_path(&path).await? {
            let commit = self.get_tree_relate_commit(tree.id, path).await?;
            let mut commit_info: LatestCommitInfo = commit.clone().into();

            // If commit has a username binding, prefer showing that username
            if let Ok(Some(binding)) = self.build_commit_binding_info(&commit.id.to_string()).await
                && !binding.is_anonymous
                && binding.matched_username.is_some()
            {
                let username = binding.matched_username.unwrap();
                commit_info.author = username.clone();
                commit_info.committer = username;
            }

            return Ok(commit_info);
        }

        // 2) If not a directory, try as file path
        // basic validation for file path
        path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;
        let parent = path
            .parent()
            .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;

        // parent must be a directory tree that exists
        if self.search_tree_by_path(parent).await?.is_none() {
            return Err(GitError::CustomError(
                "can't find target parent tree under latest commit".to_string(),
            ));
        };
        match self.resolve_latest_commit_for_file_path(&path).await? {
            Some(commit) => {
                let mut commit_info: LatestCommitInfo = commit.clone().into();
                // If commit has a username binding, prefer showing that username
                if let Ok(Some(binding)) =
                    self.build_commit_binding_info(&commit.id.to_string()).await
                    && !binding.is_anonymous
                    && binding.matched_username.is_some()
                {
                    let username = binding.matched_username.unwrap();
                    commit_info.author = username.clone();
                    commit_info.committer = username;
                }
                Ok(commit_info)
            }
            None => Err(GitError::CustomError(
                "[code:404] File not found".to_string(),
            )),
        }
    }

    /// Build commit binding information for a given commit SHA
    async fn build_commit_binding_info(
        &self,
        commit_sha: &str,
    ) -> Result<Option<CommitBindingInfo>, GitError> {
        let storage = self.get_context();
        let commit_binding_storage = storage.commit_binding_storage();

        if let Ok(Some(binding_model)) = commit_binding_storage.find_by_sha(commit_sha).await {
            Ok(Some(CommitBindingInfo {
                matched_username: binding_model.matched_username,
                is_anonymous: binding_model.is_anonymous,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_tree_info(
        &self,
        path: &Path,
        refs: Option<&str>,
    ) -> Result<Vec<TreeBriefItem>, GitError> {
        match self.search_tree_by_path_with_refs(path, refs).await? {
            Some(tree) => {
                let items = tree
                    .tree_items
                    .into_iter()
                    .map(|item| {
                        let full_path = path.join(&item.name);
                        let mut info: TreeBriefItem = item.into();
                        info.path = full_path.to_str().unwrap().to_owned();
                        info
                    })
                    .collect();
                Ok(items)
            }
            None => Ok(vec![]),
        }
    }

    async fn get_tree_commit_info(
        &self,
        path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Vec<TreeCommitItem>, GitError> {
        tracing::debug!("get_tree_commit_info called with path: {:?}, refs: {:?}", path, refs);
        
        let maybe = refs.unwrap_or("").trim();
        
        if !maybe.is_empty() && (maybe.starts_with("refs/tags/") || !maybe.contains('/')) {
            tracing::debug!("Tag browsing detected: '{}', using default behavior for individual file commits", maybe);
        } else if !maybe.is_empty() {
            tracing::debug!("Refs provided but not a tag: '{}', using default behavior", maybe);
        } else {
            tracing::debug!("No refs provided, using default behavior");
        }
        
        let commit_map = self.item_to_commit_map(path).await?;
        let mut items: Vec<TreeCommitItem> =
            commit_map.into_iter().map(TreeCommitItem::from).collect();
        items.sort_by(|a, b| {
            a.content_type
                .cmp(&b.content_type)
                .then(a.name.cmp(&b.name))
        });
        tracing::debug!("Default behavior returning {} items", items.len());
        Ok(items)
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
        let cache = Arc::new(Mutex::new(GitObjectCache::default()));
        let root_commit = Arc::new(self.get_root_commit().await);

        let Some(tree) = self.search_tree_by_path(&path).await? else {
            return Ok(HashMap::new());
        };

        let mut result = HashMap::with_capacity(tree.tree_items.len());
        for item in tree.tree_items {
            let commit = self
                .traverse_commit_history(&path, root_commit.clone(), &item, cache.clone())
                .await?;
            result.insert(item, Some(commit));
        }

        Ok(result)
    }

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
            let root_tree = self.get_tree_from_cache(commit.tree_id, &cache).await?;

            let reachable = self
                .reachable_in_tree(root_tree, path, search_item, &cache)
                .await?;

            if reachable {
                for &p_id in &commit.parent_commit_ids {
                    if !visited.contains(&p_id) {
                        let p_commit = self.get_commit_from_cache(p_id, &cache).await?;
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
        cache: &Arc<Mutex<GitObjectCache>>,
    ) -> Result<Arc<Tree>, GitError> {
        {
            let cache_guard = cache.lock().await;
            if let Some(tree) = cache_guard.trees.get(&oid) {
                return Ok(tree.clone());
            }
        }

        let tree = Arc::new(self.get_tree_by_hash(&oid.to_string()).await);

        {
            let mut cache_guard = cache.lock().await;
            cache_guard.trees.insert(oid, tree.clone());
        }
        Ok(tree)
    }

    async fn get_commit_from_cache(
        &self,
        oid: SHA1,
        cache: &Arc<Mutex<GitObjectCache>>,
    ) -> Result<Arc<Commit>, GitError> {
        {
            let cache_guard = cache.lock().await;
            if let Some(commit) = cache_guard.commits.get(&oid) {
                return Ok(commit.clone());
            }
        }

        let commit: Arc<Commit> = Arc::new(self.get_commit_by_hash(&oid.to_string()).await);

        {
            let mut cache_guard = cache.lock().await;
            cache_guard.commits.insert(oid, commit.clone());
        }
        Ok(commit)
    }

    async fn reachable_in_tree(
        &self,
        root_tree: Arc<Tree>,
        path: &Path,
        search_item: &TreeItem,
        cache: &Arc<Mutex<GitObjectCache>>,
    ) -> Result<bool, GitError> {
        let relative_path = self
            .strip_relative(path)
            .map_err(|e| GitError::CustomError(e.to_string()))?;
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

    async fn get_root_commit(&self) -> Commit;

    /// Refs-aware version; default fallback to refs-unaware implementation
    async fn item_to_commit_map_with_refs(
        &self,
        path: PathBuf,
        _refs: Option<&str>,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
        self.item_to_commit_map(path).await
    }

    /// Precise algorithm: walk commit history from HEAD and return the newest commit
    /// where the file blob at `path` differs from its parent (or was added).
    /// Returns Ok(Some(commit)) on success, Ok(None) if file not found at HEAD.
    async fn resolve_latest_commit_for_file_path(
        &self,
        path: &Path,
    ) -> Result<Option<Commit>, GitError> {
        // Ensure file exists at HEAD and capture its blob id
        let head_tree = self.get_root_tree().await;
        let head_commit = self
            .get_tree_relate_commit(head_tree.id, PathBuf::from("/"))
            .await?;

        let current_blob = self
            .get_file_blob_id(path, Some(&head_commit.id.to_string()))
            .await?;
        let Some(mut curr_blob) = current_blob else {
            return Ok(None);
        };

        let mut curr_commit = head_commit.clone();
        // Safety guard to avoid pathological loops on very deep histories
        let mut steps: u32 = 0;
        const MAX_STEPS: u32 = 10_000;

        loop {
            steps += 1;
            if steps > MAX_STEPS {
                // Fallback: give up and return HEAD commit to avoid timeouts
                tracing::warn!(
                    "resolve_latest_commit_for_file_path hit MAX_STEPS for path: {:?}",
                    path
                );
                return Ok(Some(curr_commit));
            }

            // Single-parent traversal (our commits are linear fast-forward in Mono)
            let parent_id_opt = curr_commit.parent_commit_ids.first().cloned();
            let Some(parent_id) = parent_id_opt else {
                // Reached root of history; current commit introduced the file or first reference
                return Ok(Some(curr_commit));
            };

            let parent_commit = self.get_commit_by_hash(&parent_id.to_string()).await;
            let parent_blob = self
                .get_file_blob_id(path, Some(&parent_commit.id.to_string()))
                .await?;

            if parent_blob.is_none() {
                // File did not exist in parent, so current commit added it
                return Ok(Some(curr_commit));
            }
            let p_blob = parent_blob.unwrap();
            if p_blob != curr_blob {
                // Blob changed between parent and current -> current touched the path
                return Ok(Some(curr_commit));
            }
            // Otherwise continue walking back
            curr_commit = parent_commit;
            curr_blob = p_blob;
        }
    }

    // Tag related operations shared across mono/import implementations.
    /// Create a tag in the repository context represented by `repo_path`.
    /// Returns TagInfo on success.
    async fn create_tag(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_name: Option<String>,
        tagger_email: Option<String>,
        message: Option<String>,
    ) -> Result<TagInfo, GitError>;

    /// List tags under the repository context represented by `repo_path`.
    /// Returns (items, total_count) according to Pagination.
    async fn list_tags(
        &self,
        repo_path: Option<String>,
        pagination: Pagination,
    ) -> Result<(Vec<TagInfo>, u64), GitError>;

    /// Get a tag by name under the repository context represented by `repo_path`.
    async fn get_tag(
        &self,
        repo_path: Option<String>,
        name: String,
    ) -> Result<Option<TagInfo>, GitError>;

    /// Delete a tag by name under the repository context represented by `repo_path`.
    async fn delete_tag(&self, repo_path: Option<String>, name: String) -> Result<(), GitError>;
    /// Get blame information for a file
    async fn get_file_blame(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        query: BlameQuery,
    ) -> Result<BlameResult, GitError>;

    /// Convenience: get file blob oid at HEAD (or provided refs) by path
    async fn get_file_blob_id(
        &self,
        path: &Path,
        refs: Option<&str>,
    ) -> Result<Option<SHA1>, GitError> {
        let parent = path.parent().unwrap_or(Path::new("/"));
        if let Some(tree) = self.search_tree_by_path_with_refs(parent, refs).await? {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if let Some(item) = tree.tree_items.into_iter().find(|x| x.name == name)
                && item.mode == TreeItemMode::Blob
            {
                return Ok(Some(item.id));
            }
        }
        Ok(None)
    }

    /// Preview unified diff for a single file change
    async fn preview_file_diff(
        &self,
        payload: DiffPreviewPayload,
    ) -> Result<Option<neptune::model::diff_model::DiffItem>, GitError> {
        use neptune::neptune_engine::Diff;
        let path = PathBuf::from(&payload.path);
        // old oid and content
        let old_oid_opt = self
            .get_file_blob_id(&path, Some(payload.refs.as_str()))
            .await?;
        let old_entry = if let Some(oid) = old_oid_opt {
            vec![(path.clone(), oid)]
        } else {
            Vec::new()
        };
        let new_blob = git_internal::internal::object::blob::Blob::from_content(&payload.content);
        let new_entry = vec![(path.clone(), new_blob.id)];

        // local content reader: use DB for old oid and memory for new
        let mut cache: std::collections::HashMap<SHA1, Vec<u8>> = std::collections::HashMap::new();
        if let Some(oid) = old_oid_opt
            && let Some(model) = self.get_raw_blob_by_hash(&oid.to_string()).await?
        {
            cache.insert(oid, model.data.unwrap_or_default());
        }
        cache.insert(new_blob.id, payload.content.into_bytes());

        let read =
            |_: &PathBuf, oid: &SHA1| -> Vec<u8> { cache.get(oid).cloned().unwrap_or_default() };
        let mut items =
            Diff::diff(old_entry, new_entry, "histogram".into(), Vec::new(), read).await;
        Ok(items.pop())
    }

    /// Save file edit with conflict detection and commit creation.
    async fn save_file_edit(&self, payload: EditFilePayload) -> Result<EditFileResult, GitError>;

    /// the dir's hash as same as old,file's hash is the content hash
    /// may think about change dir'hash as the content
    /// for now,only change the file's hash
    async fn get_tree_content_hash(
        &self,
        path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Vec<TreeHashItem>, GitError> {
        match self.search_tree_by_path_with_refs(&path, refs).await? {
            Some(tree) => {
                let mut items: Vec<TreeHashItem> = tree
                    .tree_items
                    .into_iter()
                    .map(TreeHashItem::from)
                    .collect();

                // sort with type and name
                items.sort_by(|a, b| {
                    a.content_type
                        .cmp(&b.content_type)
                        .then(a.name.cmp(&b.name))
                });
                Ok(items)
            }
            None => Ok(Vec::new()),
        }
    }

    /// return the dir's hash only
    async fn get_tree_dir_hash(
        &self,
        path: PathBuf,
        dir_name: &str,
        refs: Option<&str>,
    ) -> Result<Vec<TreeHashItem>, GitError> {
        match self.search_tree_by_path_with_refs(&path, refs).await? {
            Some(tree) => {
                let items: Vec<TreeHashItem> = tree
                    .tree_items
                    .into_iter()
                    .filter(|x| x.mode == TreeItemMode::Tree && x.name == dir_name)
                    .map(TreeHashItem::from)
                    .collect();
                Ok(items)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Searches for a tree in the Git repository by its path and returns the trees involved in the update and the target tree.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path to search for.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A vector of trees involved in the update process.
    /// - The target tree found at the end of the search.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the path does not exist.
    async fn search_tree_for_update(&self, path: &Path) -> Result<Vec<Arc<Tree>>, GitError> {
        // strip repo root prefix
        let relative_path = self
            .strip_relative(path)
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let root_tree = self.get_root_tree().await;

        // init state
        let mut current_tree = Arc::new(root_tree.clone());
        let mut update_chain = vec![Arc::new(root_tree)];

        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();

                // lookup child
                let search_res = current_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name)
                    .ok_or_else(|| {
                        GitError::CustomError(format!(
                            "Path '{}' not exist, please create path first!",
                            target_name
                        ))
                    })?;
                // fetch next tree
                current_tree = Arc::new(self.get_tree_by_hash(&search_res.id.to_string()).await);
                update_chain.push(current_tree.clone());
            }
        }
        Ok(update_chain)
    }

    /// Searches for a tree by a given path.
    ///
    /// This function takes a `path` and searches for the corresponding tree
    /// in the repository. It returns a `Result` containing an `Option<Tree>`.
    /// If the tree is found, it returns `Some(Tree)`. If the path does not
    /// exist, it returns `None`. In case of an error, it returns a `GitError`.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the `Path` to search for the tree.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Tree>, GitError>` - A result containing an optional tree or a Git error.
    async fn search_tree_by_path(&self, path: &Path) -> Result<Option<Tree>, GitError> {
        let relative_path = self
            .strip_relative(path)
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);
                if let Some(search_res) = search_res {
                    if !search_res.is_tree() {
                        return Ok(None);
                    }
                    let res = self.get_tree_by_hash(&search_res.id.to_string()).await;
                    search_tree = res.clone();
                } else {
                    return Ok(None);
                }
            }
        }
        Ok(Some(search_tree))
    }

    /// Get root tree for a given refs (commit SHA or tag name). If refs is None/empty, use default root.
    async fn get_root_tree_for_refs(&self, refs: Option<&str>) -> Result<Tree, GitError> {
        let maybe = refs.unwrap_or("").trim();
        if maybe.is_empty() {
            return Ok(self.get_root_tree().await);
        }
        let is_hex_sha1 = maybe.len() == 40 && maybe.chars().all(|c| c.is_ascii_hexdigit());
        let mut commit_hash = String::new();
        if is_hex_sha1 {
            commit_hash = maybe.to_string();
        } else if let Ok(Some(tag)) = self.get_tag(None, maybe.to_string()).await {
            commit_hash = tag.object_id;
        }

        if commit_hash.is_empty() {
            return Err(GitError::CustomError(
                "Invalid refs: tag or commit not found".to_string(),
            ));
        }

        let commit = self.get_commit_by_hash(&commit_hash).await;
        Ok(self.get_tree_by_hash(&commit.tree_id.to_string()).await)
    }

    /// Refs-aware tree search using a resolved root from refs
    async fn search_tree_by_path_with_refs(
        &self,
        path: &Path,
        refs: Option<&str>,
    ) -> Result<Option<Tree>, GitError> {
        let relative_path = self
            .strip_relative(path)
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let root_tree = self.get_root_tree_for_refs(refs).await?;
        let mut search_tree = root_tree.clone();
        for component in relative_path.components() {
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);
                if let Some(search_res) = search_res {
                    if !search_res.is_tree() {
                        return Ok(None);
                    }
                    let res = self.get_tree_by_hash(&search_res.id.to_string()).await;
                    search_tree = res.clone();
                } else {
                    return Ok(None);
                }
            }
        }
        Ok(Some(search_tree))
    }

    /// Searches for a tree in the Git repository by its path, creating intermediate trees if necessary,
    /// and returns the trees involved in the update process.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path to search for.
    ///
    /// # Returns
    ///
    /// A vector of trees involved in the update process.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if an error occurs during the search or tree creation process.
    async fn search_and_create_tree(&self, path: &Path) -> Result<VecDeque<Tree>, MegaError> {
        let relative_path = self.strip_relative(path).unwrap();
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        let mut update_item_tree = VecDeque::new();
        update_item_tree.push_back((root_tree, Component::RootDir));
        let mut saving_trees = VecDeque::new();
        let mut stack: VecDeque<_> = VecDeque::new();

        for component in relative_path.components() {
            if component == Component::RootDir {
                continue;
            }

            let target_name = component.as_os_str().to_str().unwrap();
            if let Some(search_res) = search_tree
                .tree_items
                .iter()
                .find(|x| x.name == target_name)
            {
                search_tree = self.get_tree_by_hash(&search_res.id.to_string()).await;
                update_item_tree.push_back((search_tree.clone(), component));
            } else {
                stack.push_back(component);
            }
        }

        let blob = generate_git_keep_with_timestamp();
        let mut last_tree = Tree::from_tree_items(vec![TreeItem {
            mode: TreeItemMode::Blob,
            id: blob.id,
            name: String::from(".gitkeep"),
        }])
        .unwrap();
        let mut last_tree_name = "";
        let mut first_element = true;

        while let Some(component) = stack.pop_back() {
            if first_element {
                first_element = false;
            } else {
                last_tree = Tree::from_tree_items(vec![TreeItem {
                    mode: TreeItemMode::Tree,
                    id: last_tree.id,
                    name: last_tree_name.to_owned(),
                }])
                .unwrap();
            }
            saving_trees.push_back(last_tree.clone());
            last_tree_name = component.as_os_str().to_str().unwrap();
        }

        if let Some((mut new_item_tree, search_name_component)) = update_item_tree.pop_back() {
            new_item_tree.tree_items.push(TreeItem {
                mode: TreeItemMode::Tree,
                id: last_tree.id,
                name: last_tree_name.to_owned(),
            });
            last_tree = Tree::from_tree_items(new_item_tree.tree_items).unwrap();
            saving_trees.push_back(last_tree.clone());

            let mut replace_hash = last_tree.id;
            let mut search_name = search_name_component.as_os_str().to_str().unwrap();
            while let Some((mut tree, component)) = update_item_tree.pop_back() {
                if let Some(index) = tree.tree_items.iter().position(|x| x.name == search_name) {
                    tree.tree_items[index].id = replace_hash;
                    let new_tree = Tree::from_tree_items(tree.tree_items).unwrap();
                    replace_hash = new_tree.id;
                    search_name = component.as_os_str().to_str().unwrap();
                    saving_trees.push_back(new_tree);
                }
            }
        }

        Ok(saving_trees)
    }

    async fn get_latest_commit_with_refs(
        &self,
        path: PathBuf,
        _refs: Option<&str>,
    ) -> Result<LatestCommitInfo, GitError> {
        // Default implementation: fallback to the version without refs
        self.get_latest_commit(path).await
    }
}
