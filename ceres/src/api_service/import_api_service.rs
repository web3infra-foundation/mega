use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::Mutex;

use async_trait::async_trait;
use git_internal::errors::GitError;
use git_internal::hash::SHA1;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use git_internal::internal::pack::entry::Entry;

use common::errors::MegaError;
use common::model::TagInfo;
use jupiter::storage::Storage;

use crate::api_service::{ApiHandler, GitObjectCache};
use crate::model::blame::{BlameQuery, BlameResult};
use crate::model::git::{CreateEntryInfo, EditFilePayload, EditFileResult};
use crate::protocol::repo::Repo;
use callisto::{git_tag, import_refs};
use jupiter::utils::converter::FromGitModel;

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

    async fn create_monorepo_entry(&self, _: CreateEntryInfo) -> Result<(), GitError> {
        return Err(GitError::CustomError(
            "import dir does not support create entry".to_string(),
        ));
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, MegaError> {
        if let Ok(relative_path) = path.strip_prefix(self.repo.repo_path.clone()) {
            Ok(relative_path.to_path_buf())
        } else {
            Err(MegaError::with_message(
                "The full path does not start with the base path.",
            ))
        }
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
        Tree::from_git_model(
            storage
                .get_tree_by_hash(self.repo.repo_id, &root_commit.tree)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        Tree::from_git_model(
            self.storage
                .git_db_storage()
                .get_tree_by_hash(self.repo.repo_id, hash)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        let storage = self.storage.git_db_storage();
        let commit = storage
            .get_commit_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap();
        commit.map(Commit::from_git_model)
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
        Ok(commits.into_iter().map(Commit::from_git_model).collect())
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

    async fn create_tag(
        &self,
        _repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_name: Option<String>,
        tagger_email: Option<String>,
        message: Option<String>,
    ) -> Result<TagInfo, GitError> {
        let git_storage = self.storage.git_db_storage();
        let is_annotated = message.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let tagger_info = match (tagger_name, tagger_email) {
            (Some(n), Some(e)) => format!("{} <{}>", n, e),
            (Some(n), None) => n,
            (None, Some(e)) => e,
            (None, None) => "unknown".to_string(),
        };

        // validate target commit if provided
        self.validate_target_commit(target.as_ref()).await?;

        let full_ref = format!("refs/tags/{}", name.clone());
        // Prevent duplicate tag/ref creation: check annotated table and refs first.
        match git_storage
            .get_tag_by_repo_and_name(self.repo.repo_id, &name)
            .await
        {
            Ok(Some(_)) => {
                return Err(GitError::CustomError(format!(
                    "[code:400] Tag '{}' already exists",
                    name
                )));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while checking git_tag existence: {}", e);
                return Err(GitError::CustomError("[code:500] DB error".to_string()));
            }
        }

        if let Ok(refs) = git_storage.get_ref(self.repo.repo_id).await
            && refs.iter().any(|r| r.ref_name == full_ref)
        {
            return Err(GitError::CustomError(format!(
                "[code:400] Tag '{}' already exists",
                name
            )));
        }
        if is_annotated {
            return self
                .create_annotated_tag(&git_storage, full_ref, name, target, tagger_info, message)
                .await;
        }

        // lightweight
        self.create_lightweight_tag(&git_storage, full_ref, name, target, tagger_info)
            .await
    }

    async fn list_tags(
        &self,
        _repo_path: Option<String>,
        pagination: common::model::Pagination,
    ) -> Result<(Vec<TagInfo>, u64), GitError> {
        let git_storage = self.storage.git_db_storage();
        // annotated tags: fetch paged annotated tags from storage
        let (annotated_tags_page, annotated_total) = match git_storage
            .list_tags_by_repo_with_page(self.repo.repo_id, pagination.clone())
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("DB error while listing git tags: {}", e);
                return Err(GitError::CustomError("[code:500] DB error".to_string()));
            }
        };

        // map annotated page into TagInfo
        let mut result: Vec<TagInfo> = annotated_tags_page
            .into_iter()
            .map(|t| TagInfo {
                name: t.tag_name,
                tag_id: t.tag_id,
                object_id: t.object_id,
                object_type: t.object_type,
                tagger: t.tagger,
                message: t.message,
                created_at: t.created_at.and_utc().to_rfc3339(),
            })
            .collect();

        // lightweight refs
        let mut lightweight_refs: Vec<TagInfo> = vec![];
        if let Ok(refs) = git_storage.get_ref(self.repo.repo_id).await {
            for r in refs {
                if r.ref_name.starts_with("refs/tags/") {
                    let tag_name = r.ref_name.trim_start_matches("refs/tags/").to_string();
                    // skip if annotated exists (anywhere)
                    // Note: we only have the annotated page in memory; to avoid duplicate names we check by tag_name against annotated page and will accept duplicates only if not present.
                    if result.iter().any(|t| t.name == tag_name) {
                        continue;
                    }
                    let created_at = r.created_at.and_utc().to_rfc3339();
                    lightweight_refs.push(TagInfo {
                        name: tag_name.clone(),
                        tag_id: r.ref_git_id.clone(),
                        object_id: r.ref_git_id.clone(),
                        object_type: "commit".to_string(),
                        tagger: "".to_string(),
                        message: "".to_string(),
                        created_at,
                    });
                }
            }
        }

        // total is annotated_total + lightweight_refs.len()
        let total = annotated_total + lightweight_refs.len() as u64;

        // fill page: annotated page items come first, then lightweight refs to make up page size
        let per_page = if pagination.per_page == 0 {
            20
        } else {
            pagination.per_page
        } as usize;
        if result.len() < per_page {
            let need = per_page - result.len();
            for r in lightweight_refs.into_iter().take(need) {
                result.push(r);
            }
        }

        Ok((result, total))
    }

    async fn get_tag(
        &self,
        _repo_path: Option<String>,
        name: String,
    ) -> Result<Option<TagInfo>, GitError> {
        let git_storage = self.storage.git_db_storage();
        // annotated first: use jupiter git_storage helper
        match git_storage
            .get_tag_by_repo_and_name(self.repo.repo_id, &name)
            .await
        {
            Ok(Some(tag)) => {
                return Ok(Some(TagInfo {
                    name: tag.tag_name,
                    tag_id: tag.tag_id,
                    object_id: tag.object_id,
                    object_type: tag.object_type,
                    tagger: tag.tagger,
                    message: tag.message,
                    created_at: tag.created_at.and_utc().to_rfc3339(),
                }));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while getting git tag: {}", e);
                return Err(GitError::CustomError("[code:500] DB error".to_string()));
            }
        }
        // check import_refs for lightweight
        let full_ref = format!("refs/tags/{}", name.clone());
        if let Ok(refs) = git_storage.get_ref(self.repo.repo_id).await {
            for r in refs {
                if r.ref_name == full_ref {
                    let created_at = r.created_at.and_utc().to_rfc3339();
                    return Ok(Some(TagInfo {
                        name: name.clone(),
                        tag_id: r.ref_git_id.clone(),
                        object_id: r.ref_git_id.clone(),
                        object_type: "commit".to_string(),
                        tagger: "".to_string(),
                        message: "".to_string(),
                        created_at,
                    }));
                }
            }
        }
        Ok(None)
    }

    async fn delete_tag(&self, _repo_path: Option<String>, name: String) -> Result<(), GitError> {
        let git_storage = self.storage.git_db_storage();
        // annotated first: use jupiter helpers
        match git_storage
            .get_tag_by_repo_and_name(self.repo.repo_id, &name)
            .await
        {
            Ok(Some(_tag)) => {
                // remove import ref if exists
                let full_ref = format!("refs/tags/{}", name.clone());
                git_storage
                    .remove_ref(self.repo.repo_id, &full_ref)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "Failed to remove import ref when deleting annotated tag: {}",
                            e
                        );
                        GitError::CustomError("[code:500] Failed to remove import ref".to_string())
                    })?;
                git_storage
                    .delete_tag(self.repo.repo_id, &name)
                    .await
                    .map_err(|e| {
                        tracing::error!("DB delete error when deleting annotated git tag: {}", e);
                        GitError::CustomError("[code:500] DB delete error".to_string())
                    })?;
                Ok(())
            }
            Ok(None) => {
                // remove lightweight ref if exists
                let full_ref = format!("refs/tags/{}", name.clone());
                // try remove
                git_storage
                    .remove_ref(self.repo.repo_id, &full_ref)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "Failed to remove import ref when deleting lightweight tag: {}",
                            e
                        );
                        GitError::CustomError("[code:500] Failed to remove import ref".to_string())
                    })?;
                Ok(())
            }
            Err(e) => Err(GitError::CustomError(format!("[code:500] DB error: {}", e))),
        }
    }

    async fn get_file_blame(
        &self,
        _file_path: &str,
        _ref_name: Option<&str>,
        _query: BlameQuery,
    ) -> Result<BlameResult, GitError> {
        Err(GitError::CustomError(
            "Import directory does not support blame functionality".to_string(),
        ))
    }

    /// Save file edit for import repo path
    async fn save_file_edit(&self, payload: EditFilePayload) -> Result<EditFileResult, GitError> {
        use git_internal::internal::object::blob::Blob;
        use git_internal::internal::object::tree::TreeItemMode;

        let path = PathBuf::from(&payload.path);
        let parent = path
            .parent()
            .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;
        let update_chain = self
            .search_tree_for_update(parent)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let parent_tree = update_chain
            .last()
            .cloned()
            .ok_or_else(|| GitError::CustomError("Parent tree not found".to_string()))?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::CustomError("Invalid file name".to_string()))?;
        let _current = parent_tree
            .tree_items
            .iter()
            .find(|x| x.name == name && x.mode == TreeItemMode::Blob)
            .ok_or_else(|| GitError::CustomError("[code:404] File not found".to_string()))?;

        // Create new blob and rebuild tree up to root
        let new_blob = Blob::from_content(&payload.content);
        let (updated_trees, new_root_id) =
            self.build_updated_trees(path.clone(), update_chain, new_blob.id)?;

        // Save commit and objects under import repo tables
        let git_storage = self.storage.git_db_storage();
        let new_commit_id = {
            // Update default branch ref commit with parent = current default commit
            let default_ref = git_storage
                .get_default_ref(self.repo.repo_id)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?
                .ok_or_else(|| GitError::CustomError("Default ref not found".to_string()))?;
            let current_commit = git_storage
                .get_commit_by_hash(self.repo.repo_id, &default_ref.ref_git_id)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?
                .ok_or(GitError::InvalidCommitObject)?;
            let parent_id = SHA1::from_str(&current_commit.commit_id).unwrap();

            let new_commit =
                Commit::from_tree_id(new_root_id, vec![parent_id], &payload.commit_message);
            let new_commit_id = new_commit.id.to_string();

            let mut entries: Vec<Entry> = Vec::new();
            for t in updated_trees.iter().cloned() {
                entries.push(Entry::from(t));
            }
            entries.push(Entry::from(new_blob.clone()));
            entries.push(Entry::from(new_commit.clone()));
            git_storage
                .save_entry(self.repo.repo_id, entries)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;

            // Update ref to new commit id
            git_storage
                .update_ref(self.repo.repo_id, &default_ref.ref_name, &new_commit_id)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
            new_commit_id
        };

        Ok(EditFileResult {
            commit_id: new_commit_id,
            new_oid: new_blob.id.to_string(),
            path: payload.path,
        })
    }
}

impl ImportApiService {
    async fn create_annotated_tag(
        &self,
        git_storage: &jupiter::storage::git_db_storage::GitDbStorage,
        full_ref: String,
        name: String,
        target: Option<String>,
        tagger_info: String,
        message: Option<String>,
    ) -> Result<TagInfo, GitError> {
        // build git_internal tag and models
        let (tag_id_hex, object_id) = self.build_git_internal_tag(
            name.clone(),
            target.clone(),
            tagger_info.clone(),
            message.clone(),
        )?;

        let new_model = self.build_git_tag_model(
            tag_id_hex.clone(),
            object_id.clone(),
            name.clone(),
            tagger_info.clone(),
            message.clone(),
        );
        match git_storage.insert_tag(new_model).await {
            Ok(saved) => {
                // write import ref; rollback handled inside helper
                self.write_import_ref_with_rollback(
                    full_ref.clone(),
                    object_id.clone(),
                    self.repo.repo_id,
                    &name,
                )
                .await?;
                Ok(TagInfo {
                    name: saved.tag_name,
                    tag_id: saved.tag_id,
                    object_id: saved.object_id,
                    object_type: saved.object_type,
                    tagger: saved.tagger,
                    message: saved.message,
                    created_at: saved.created_at.and_utc().to_rfc3339(),
                })
            }
            Err(e) => {
                tracing::error!("DB insert error when creating annotated git tag: {}", e);
                Err(GitError::CustomError(
                    "[code:500] DB insert error".to_string(),
                ))
            }
        }
    }

    async fn create_lightweight_tag(
        &self,
        git_storage: &jupiter::storage::git_db_storage::GitDbStorage,
        full_ref: String,
        name: String,
        target: Option<String>,
        tagger_info: String,
    ) -> Result<TagInfo, GitError> {
        let object_id = target.clone().unwrap_or_default();
        let import_ref = import_refs::Model {
            id: common::utils::generate_id(),
            repo_id: self.repo.repo_id,
            ref_name: full_ref.clone(),
            ref_git_id: object_id.clone(),
            ref_type: callisto::sea_orm_active_enums::RefTypeEnum::Tag,
            default_branch: false,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        // Use ref creation time as lightweight tag created_at (capture before move)
        let created_at = import_ref.created_at.and_utc().to_rfc3339();
        git_storage
            .save_ref(self.repo.repo_id, import_ref)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write import ref for lightweight tag: {}", e);
                GitError::CustomError("[code:500] Failed to write import ref".to_string())
            })?;
        Ok(TagInfo {
            name: name.clone(),
            tag_id: object_id.clone(),
            object_id: object_id.clone(),
            object_type: "commit".to_string(),
            tagger: tagger_info.clone(),
            message: String::new(),
            created_at,
        })
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

    async fn get_root_commit(&self) -> Commit {
        let storage = self.storage.git_db_storage();
        let refs = storage
            .get_default_ref(self.repo.repo_id)
            .await
            .unwrap()
            .unwrap();
        Commit::from_git_model(
            storage
                .get_commit_by_hash(self.repo.repo_id, &refs.ref_git_id)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn validate_target_commit(&self, target: Option<&String>) -> Result<(), GitError> {
        if let Some(ref t) = target {
            let git_storage = self.storage.git_db_storage();
            match git_storage.get_commit_by_hash(self.repo.repo_id, t).await {
                Ok(c) => {
                    if c.is_none() {
                        return Err(GitError::CustomError(format!(
                            "[code:404] Target commit '{}' not found",
                            t
                        )));
                    }
                }
                Err(e) => {
                    tracing::error!("DB error while fetching commit by hash: {}", e);
                    return Err(GitError::CustomError("[code:500] DB error".to_string()));
                }
            }
        }
        Ok(())
    }

    fn build_git_internal_tag(
        &self,
        name: String,
        target: Option<String>,
        tagger_info: String,
        message: Option<String>,
    ) -> Result<(String, String), GitError> {
        let tag_target = target
            .as_ref()
            .ok_or(GitError::InvalidCommitObject)
            .and_then(|t| SHA1::from_str(t).map_err(|_| GitError::InvalidCommitObject))?;
        let git_internal_tag = git_internal::internal::object::tag::Tag::new(
            tag_target,
            git_internal::internal::object::types::ObjectType::Commit,
            name.clone(),
            git_internal::internal::object::signature::Signature::new(
                git_internal::internal::object::signature::SignatureType::Tagger,
                tagger_info.clone(),
                String::new(),
            ),
            message.clone().unwrap_or_default(),
        );
        Ok((
            git_internal_tag.id.to_string(),
            target.unwrap_or_else(|| "HEAD".to_string()),
        ))
    }

    fn build_git_tag_model(
        &self,
        tag_id_hex: String,
        object_id: String,
        name: String,
        tagger_info: String,
        message: Option<String>,
    ) -> git_tag::Model {
        git_tag::Model {
            id: common::utils::generate_id(),
            repo_id: self.repo.repo_id,
            tag_id: tag_id_hex,
            object_id,
            object_type: "commit".to_string(),
            tag_name: name,
            tagger: tagger_info,
            message: message.unwrap_or_default(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    async fn write_import_ref_with_rollback(
        &self,
        full_ref: String,
        object_id: String,
        repo_id: i64,
        tag_name: &str,
    ) -> Result<(), GitError> {
        let git_storage = self.storage.git_db_storage();
        let import_ref = import_refs::Model {
            id: common::utils::generate_id(),
            repo_id,
            ref_name: full_ref.clone(),
            ref_git_id: object_id.clone(),
            ref_type: callisto::sea_orm_active_enums::RefTypeEnum::Tag,
            default_branch: false,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        if let Err(e) = git_storage.save_ref(repo_id, import_ref).await {
            if let Err(del_e) = git_storage.delete_tag(repo_id, tag_name).await {
                tracing::error!(
                    "Failed to rollback git_tag DB record after ref write failure: {}",
                    del_e
                );
            }
            tracing::error!("Failed to write import ref after DB insert: {}", e);
            return Err(GitError::CustomError(
                "[code:500] Failed to write import ref".to_string(),
            ));
        }
        Ok(())
    }

    fn update_tree_hash(
        &self,
        tree: Arc<Tree>,
        name: &str,
        target_hash: SHA1,
    ) -> Result<Tree, GitError> {
        let index = tree
            .tree_items
            .iter()
            .position(|item| item.name == name)
            .ok_or_else(|| GitError::CustomError(format!("Tree item '{}' not found", name)))?;
        let mut items = tree.tree_items.clone();
        items[index].id = target_hash;
        Tree::from_tree_items(items).map_err(|_| GitError::CustomError("Invalid tree".to_string()))
    }

    /// Build updated trees chain and return (updated_trees, new_root_tree_id)
    fn build_updated_trees(
        &self,
        mut path: PathBuf,
        mut update_chain: Vec<Arc<Tree>>,
        mut updated_tree_hash: SHA1,
    ) -> Result<(Vec<Tree>, SHA1), GitError> {
        let mut updated_trees = Vec::new();
        while let Some(tree) = update_chain.pop() {
            let cloned_path = path.clone();
            let name = cloned_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| GitError::CustomError("Invalid path".into()))?;
            path.pop();

            let new_tree = self.update_tree_hash(tree, name, updated_tree_hash)?;
            updated_tree_hash = new_tree.id;
            updated_trees.push(new_tree);
        }
        Ok((updated_trees, updated_tree_hash))
    }
}
