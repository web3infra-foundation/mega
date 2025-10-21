//! # Mono API Service
//!
//! This module provides the API service implementation for monorepo operations in the Mega system.
//! The `MonoApiService` struct implements the `ApiHandler` trait to provide comprehensive
//! monorepo management capabilities including file operations, merge request handling,
//! and Git-like version control functionality.
//!
//! ## Key Features
//!
//! - **File Management**: Create files and directories within the monorepo structure
//! - **Tree Operations**: Handle Git tree objects for version control
//! - **Merge Requests**: Process and merge pull/merge requests with conflict resolution
//! - **Diff Operations**: Generate content differences between commits using libra
//! - **Commit Management**: Retrieve and manage commit objects and their relationships
//! - **Storage Integration**: Seamless integration with the underlying storage layer
//!
//! ## Core Components
//!
//! - `MonoApiService`: Main service struct that wraps storage functionality
//! - `ApiHandler` implementation: Provides standardized API operations
//! - Merge request processing with automated conflict detection
//! - Tree traversal and blob extraction utilities
//!
//! ## Dependencies
//!
//! This module relies on several core components:
//! - `git_internal`: Git object handling and version control primitives
//! - `jupiter`: Storage layer abstraction and data persistence
//! - `callisto`: Database models and ORM functionality
//! - `libra`: External Git-compatible command-line tool for diff operations
//!
//! ## Usage
//!
//! The service is typically instantiated with a storage backend and used to handle
//! API requests for monorepo operations. All operations are asynchronous and return
//! appropriate error types for robust error handling.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use crate::api_service::ApiHandler;
use crate::model::blame::{BlameQuery, BlameResult};
use crate::model::change_list::ClDiffFile;
use crate::model::git::CreateEntryInfo;
use crate::model::git::{EditFilePayload, EditFileResult};
use crate::model::third_party::{ThirdPartyClient, ThirdPartyRepoTrait};
use crate::protocol::{SmartProtocol, TransportProtocol};
use async_trait::async_trait;
use bytes::Bytes;
use git_internal::errors::GitError;
use git_internal::hash::SHA1;
use git_internal::internal::object::blob::Blob;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use neptune::model::diff_model::DiffItem;
use neptune::neptune_engine::Diff;
use regex::Regex;

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_cl, mega_tag, mega_tree};
use common::errors::MegaError;
use common::model::{Pagination, TagInfo};
use jupiter::utils::converter::{FromMegaModel, IntoMegaModel};

use jupiter::service::blame_service::BlameService;
use jupiter::storage::Storage;
use jupiter::storage::base_storage::StorageConnector;
use jupiter::utils::converter::generate_git_keep_with_timestamp;

#[derive(Clone)]
pub struct MonoApiService {
    pub storage: Storage,
}

pub struct TreeUpdateResult {
    pub updated_trees: Vec<Tree>,
    pub ref_updates: Vec<RefUpdate>,
}

pub enum RefUpdate {
    Update { path: String, tree_id: SHA1 },
    Delete { path: String },
}

#[async_trait]
impl ApiHandler for MonoApiService {
    fn get_context(&self) -> Storage {
        self.storage.clone()
    }

    /// Save file edit in monorepo with optimistic concurrency check
    async fn save_file_edit(&self, payload: EditFilePayload) -> Result<EditFileResult, GitError> {
        let storage = self.storage.mono_storage();
        let file_path = PathBuf::from(&payload.path);
        let parent_path = file_path
            .parent()
            .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;

        // Build update chain to parent directory and determine current blob id
        let update_chain = self.search_tree_for_update(parent_path).await?;
        let parent_tree = update_chain
            .last()
            .ok_or_else(|| GitError::CustomError("Parent tree not found".to_string()))?
            .clone();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::CustomError("Invalid file name".to_string()))?;

        let _current_item = parent_tree
            .tree_items
            .iter()
            .find(|x| x.name == file_name && x.mode == TreeItemMode::Blob)
            .ok_or_else(|| GitError::CustomError("[code:404] File not found".to_string()))?;

        // Create new blob and build update result up to root
        let new_blob = Blob::from_content(&payload.content);
        let result = self
            .build_result_by_chain(file_path.clone(), update_chain, new_blob.id)
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        // Apply and save
        let new_commit_id = self
            .apply_update_result(&result, &payload.commit_message)
            .await?;
        storage
            .save_mega_blobs(vec![&new_blob], &new_commit_id)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        Ok(EditFileResult {
            commit_id: new_commit_id,
            new_oid: new_blob.id.to_string(),
            path: payload.path,
        })
    }

    /// Creates a new file or directory in the monorepo based on the provided file information.
    ///
    /// # Arguments
    ///
    /// * `entry_info` - Information about the file or directory to create.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `GitError` on failure.
    async fn create_monorepo_entry(&self, entry_info: CreateEntryInfo) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();
        let path = PathBuf::from(&entry_info.path);
        let mut save_trees = vec![];

        // Try to get the update chain for the given path.
        // If the path exists, return an empty missing_parts and prefix.
        // If part of the path does not exist, extract the missing segments (missing_parts),
        // determine the valid existing prefix, and rebuild the update_chain from that prefix.
        let (missing_parts, prefix, mut update_chain) =
            match self.search_tree_for_update(&path).await {
                Ok(chain) => (Vec::new(), "", chain),
                Err(err) => {
                    // If search_tree_for_update failed, try to extract the
                    // portion of the path that does not exist from the
                    // error message. The error message is expected to
                    // contain a substring like: Path '.../missing' not exist
                    // We capture that substring to determine which segments
                    // need to be created.
                    let re: Regex = Regex::new(r"Path '([^']+)' not exist").unwrap();
                    let extracted = re
                        .captures(&err.to_string())
                        .map(|caps| caps[1].to_string())
                        .unwrap_or(err.to_string());

                    // missing_parts: the trailing path segments after the
                    // first occurrence of the extracted non-existent path.
                    // Example: entry_info.path = "a/b/c/d" and extracted = "c/d"
                    // Then missing_parts = ["c", "d"]
                    let missing_parts = entry_info
                        .path
                        .find(&extracted)
                        .map(|pos| &entry_info.path[pos..])
                        .map(|sub| sub.split('/').collect::<Vec<_>>())
                        .unwrap_or_default();

                    // prefix: the valid existing path before the missing parts.
                    // Using the same example above, prefix = "a/b/"
                    let prefix = entry_info
                        .path
                        .find(&extracted)
                        .map(|pos| &entry_info.path[..pos])
                        .unwrap_or("");

                    // Rebuild the update chain starting from the valid prefix
                    // so subsequent operations only update from that known
                    // existing tree downward.
                    let chain = self.search_tree_for_update(Path::new(prefix)).await?;
                    (missing_parts, prefix, chain)
                }
            };

        let target_items = update_chain.pop().unwrap().tree_items.clone();

        // If there are no missing parts, we are inserting directly into an
        // existing tree. This branch handles both creating a new file or
        // creating a new directory in the target tree.
        if missing_parts.is_empty() {
            let mut target_items = target_items;

            // Check for duplicate
            let is_tree_mode = if entry_info.is_directory {
                TreeItemMode::Tree
            } else {
                TreeItemMode::Blob
            };
            if target_items
                .iter()
                .any(|x| x.mode == is_tree_mode && x.name == entry_info.name)
            {
                return Err(GitError::CustomError("Duplicate name".to_string()));
            }

            // Create a new tree item based on whether it's a directory or file
            let (new_item, blob) = if entry_info.is_directory {
                // For a new directory, create a .gitkeep blob so the
                // directory can be represented as a tree with at least
                // one blob entry. The blob contains a timestamp so it's
                // unique.
                let blob = generate_git_keep_with_timestamp();
                let tree_item = TreeItem {
                    mode: TreeItemMode::Blob,
                    id: blob.id,
                    name: String::from(".gitkeep"),
                };
                let new_dir_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
                save_trees.push(new_dir_tree.clone());
                (
                    TreeItem {
                        mode: TreeItemMode::Tree,
                        id: new_dir_tree.id,
                        name: entry_info.name.clone(),
                    },
                    blob,
                )
            } else {
                let blob = Blob::from_content(&entry_info.content.clone().unwrap());
                (
                    TreeItem {
                        mode: TreeItemMode::Blob,
                        id: blob.id,
                        name: entry_info.name.clone(),
                    },
                    blob,
                )
            };

            target_items.push(new_item);
            target_items.sort_by(|a, b| a.name.cmp(&b.name));
            let target_tree = Tree::from_tree_items(target_items).unwrap();
            save_trees.push(target_tree.clone());

            // Build update instructions for parent trees and refs.
            // build_result_by_chain walks the update_chain (parent trees)
            // and prepares the list of updated trees and ref updates
            // that must be applied to persist the change.
            let update_result = self.build_result_by_chain(
                if prefix.is_empty() {
                    path.clone()
                } else {
                    PathBuf::from(&prefix)
                },
                update_chain,
                target_tree.id,
            )?;
            let new_commit_id = self
                .apply_update_result(&update_result, &entry_info.commit_msg())
                .await?;

            storage.save_mega_blobs(vec![&blob], &new_commit_id).await?;

            let save_trees: Vec<mega_tree::ActiveModel> = save_trees
                .into_iter()
                .map(|save_t| {
                    let mut tree_model: mega_tree::Model = save_t.into_mega_model();
                    tree_model.commit_id.clone_from(&new_commit_id);
                    tree_model.into()
                })
                .collect();
            storage.batch_save_model(save_trees).await?;
        } else {
            // If missing_parts is not empty, we must create intermediate
            // directories (trees) for each missing segment. This branch
            // constructs the leaf tree first and then wraps it with
            // additional trees for each missing path component up to the
            // existing prefix.
            // Create a new tree item based on whether it's a directory or file
            let (leaf_item, blob) = if entry_info.is_directory {
                // Create .gitkeep blob and an initial tree for the new
                // directory leaf. This represents the directory's own
                // tree object which will be nested under new parent trees.
                let blob = generate_git_keep_with_timestamp();
                let tree_item = TreeItem {
                    mode: TreeItemMode::Blob,
                    id: blob.id,
                    name: String::from(".gitkeep"),
                };
                let new_dir_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
                save_trees.push(new_dir_tree.clone());
                (
                    TreeItem {
                        mode: TreeItemMode::Tree,
                        id: new_dir_tree.id,
                        name: entry_info.name.clone(),
                    },
                    blob,
                )
            } else {
                let blob = Blob::from_content(&entry_info.content.clone().unwrap());
                (
                    TreeItem {
                        mode: TreeItemMode::Blob,
                        id: blob.id,
                        name: entry_info.name.clone(),
                    },
                    blob,
                )
            };

            let mut current_tree = Tree::from_tree_items(vec![leaf_item]).unwrap();
            save_trees.push(current_tree.clone());

            // Wrap the leaf tree with trees for each missing parent segment.
            // We iterate the missing parts in reverse (from leaf's parent up
            // to the topmost missing segment) and create a tree object for
            // each level that points to the previously built child tree.
            let missing_len = missing_parts.len();
            for part in missing_parts.iter().rev().take(missing_len - 1) {
                let sub_item = TreeItem {
                    mode: TreeItemMode::Tree,
                    id: current_tree.id,
                    name: part.to_string(),
                };

                current_tree = Tree::from_tree_items(vec![sub_item]).unwrap();
                save_trees.push(current_tree.clone());
            }

            // top_part is the highest-level missing segment (closest to the
            // existing prefix). We'll insert this as a child into the
            // existing target_items collected from the update chain.
            let top_part = missing_parts.first().unwrap().to_string();
            let top_item = TreeItem {
                mode: TreeItemMode::Tree,
                id: current_tree.id,
                name: top_part.clone(),
            };

            let mut target_items = target_items;

            // Check for duplicate
            if target_items
                .iter()
                .any(|x| x.mode == TreeItemMode::Tree && x.name == top_part)
            {
                return Err(GitError::CustomError("Duplicate name".to_string()));
            }

            target_items.push(top_item);
            target_items.sort_by(|a, b| a.name.cmp(&b.name));
            let target_tree = Tree::from_tree_items(target_items).unwrap();
            save_trees.push(target_tree.clone());

            // After constructing the nested trees, build update instructions
            // and apply them to update the parent trees and refs so the
            // new nested directory/file is persisted in the repository.
            let update_result =
                self.build_result_by_chain(PathBuf::from(prefix), update_chain, target_tree.id)?;
            let new_commit_id = self
                .apply_update_result(&update_result, &entry_info.commit_msg())
                .await?;

            storage
                .save_mega_blobs(vec![&blob], &new_commit_id)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;

            let save_trees: Vec<mega_tree::ActiveModel> = save_trees
                .into_iter()
                .map(|save_t| {
                    let mut tree_model: mega_tree::Model = save_t.into_mega_model();
                    tree_model.commit_id.clone_from(&new_commit_id);
                    tree_model.into()
                })
                .collect();
            storage
                .batch_save_model(save_trees)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
        }

        Ok(())
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, MegaError> {
        Ok(path.to_path_buf())
    }

    async fn get_root_tree(&self) -> Tree {
        let storage = self.storage.mono_storage();
        let refs = storage.get_ref("/").await.unwrap().unwrap();

        Tree::from_mega_model(
            storage
                .get_tree_by_hash(&refs.ref_tree_hash)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        Tree::from_mega_model(
            self.storage
                .mono_storage()
                .get_tree_by_hash(hash)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        match self.storage.mono_storage().get_commit_by_hash(hash).await {
            Ok(Some(commit)) => Some(Commit::from_mega_model(commit)),
            _ => None,
        }
    }

    async fn get_tree_relate_commit(&self, t_hash: SHA1, _: PathBuf) -> Result<Commit, GitError> {
        let storage = self.storage.mono_storage();
        let tree_info = storage
            .get_tree_by_hash(&t_hash.to_string())
            .await
            .unwrap()
            .unwrap();
        Ok(Commit::from_mega_model(
            storage
                .get_commit_by_hash(&tree_info.commit_id)
                .await
                .unwrap()
                .unwrap(),
        ))
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let commits = self
            .storage
            .mono_storage()
            .get_commits_by_hashes(&c_hashes)
            .await
            .unwrap();
        Ok(commits.into_iter().map(Commit::from_mega_model).collect())
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let mut item_to_commit = HashMap::new();

                let storage = self.storage.mono_storage();
                let tree_hashes = tree
                    .tree_items
                    .iter()
                    .filter(|x| x.mode == TreeItemMode::Tree)
                    .map(|x| x.id.to_string())
                    .collect();
                let trees = storage.get_trees_by_hashes(tree_hashes).await.unwrap();
                for tree in trees {
                    item_to_commit.insert(tree.tree_id, tree.commit_id);
                }

                let blob_hashes = tree
                    .tree_items
                    .iter()
                    .filter(|x| x.mode == TreeItemMode::Blob)
                    .map(|x| x.id.to_string())
                    .collect();
                let blobs = storage.get_mega_blobs_by_hashes(blob_hashes).await.unwrap();
                for blob in blobs {
                    item_to_commit.insert(blob.blob_id, blob.commit_id);
                }

                let commit_ids: HashSet<String> = item_to_commit.values().cloned().collect();
                let commits = self
                    .get_commits_by_hashes(commit_ids.into_iter().collect())
                    .await
                    .unwrap();
                let commit_map: HashMap<String, Commit> =
                    commits.into_iter().map(|x| (x.id.to_string(), x)).collect();

                let mut result: HashMap<TreeItem, Option<Commit>> = HashMap::new();
                for item in tree.tree_items {
                    if let Some(commit_id) = item_to_commit.get(&item.id.to_string()) {
                        let commit = if let Some(commit) = commit_map.get(commit_id) {
                            Some(commit.to_owned())
                        } else {
                            tracing::warn!("failed fetch from commit map: {}", commit_id);
                            None
                        };
                        result.insert(item, commit);
                    }
                }
                Ok(result)
            }
            None => Ok(HashMap::new()),
        }
    }
    // helper to convert mega_tag model into TagInfo (defined on MonoApiService below)

    async fn create_tag(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_name: Option<String>,
        tagger_email: Option<String>,
        message: Option<String>,
    ) -> Result<TagInfo, GitError> {
        let mono_storage = self.storage.mono_storage();

        let is_annotated = message.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let tagger_info = match (tagger_name, tagger_email) {
            (Some(n), Some(e)) => format!("{} <{}>", n, e),
            (Some(n), None) => n,
            (None, Some(e)) => e,
            (None, None) => "unknown".to_string(),
        };

        // validate target commit presence
        self.validate_target_commit_mono(target.as_ref()).await?;

        let full_ref = format!("refs/tags/{}", name.clone());

        // Prevent duplicate tag/ref creation
        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(_)) => {
                return Err(GitError::CustomError(format!(
                    "[code:400] Tag '{}' already exists",
                    name
                )));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while checking tag existence: {}", e);
                return Err(GitError::CustomError("[code:500] DB error".to_string()));
            }
        }

        if let Ok(Some(_)) = mono_storage.get_ref_by_name(&full_ref).await {
            return Err(GitError::CustomError(format!(
                "[code:400] Tag '{}' already exists",
                name
            )));
        }

        if is_annotated {
            return self
                .create_annotated_tag_mono(
                    repo_path.clone(),
                    name.clone(),
                    target.clone(),
                    tagger_info.clone(),
                    message.clone(),
                    full_ref.clone(),
                )
                .await;
        }

        // lightweight
        self.create_lightweight_tag_mono(
            repo_path.clone(),
            name.clone(),
            target.clone(),
            tagger_info.clone(),
            full_ref.clone(),
        )
        .await
    }

    async fn list_tags(
        &self,
        repo_path: Option<String>,
        pagination: Pagination,
    ) -> Result<(Vec<TagInfo>, u64), GitError> {
        let mono_storage = self.storage.mono_storage();
        // annotated tags from DB (paged)
        let (annotated_page, annotated_total) =
            match mono_storage.get_tags_by_page(pagination.clone()).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("DB error while listing tags: {}", e);
                    return Err(GitError::CustomError("[code:500] DB error".to_string()));
                }
            };

        let mut result: Vec<TagInfo> = annotated_page
            .into_iter()
            .map(|t| self.tag_model_to_info(t))
            .collect();

        // lightweight refs from refs table under path
        let repo_path = repo_path.as_deref().unwrap_or("/");
        let mut lightweight_refs: Vec<TagInfo> = vec![];
        if let Ok(refs) = mono_storage.get_refs(repo_path).await {
            for r in refs {
                if r.ref_name.starts_with("refs/tags/") {
                    let tag_name = r.ref_name.trim_start_matches("refs/tags/").to_string();
                    if result.iter().any(|t| t.name == tag_name) {
                        continue;
                    }
                    lightweight_refs.push(TagInfo {
                        name: tag_name.clone(),
                        tag_id: r.ref_commit_hash.clone(),
                        object_id: r.ref_commit_hash.clone(),
                        object_type: "commit".to_string(),
                        tagger: "".to_string(),
                        message: "".to_string(),
                        created_at: r.created_at.and_utc().to_rfc3339(),
                    });
                }
            }
        }

        let total = annotated_total + lightweight_refs.len() as u64;
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
        repo_path: Option<String>,
        name: String,
    ) -> Result<Option<TagInfo>, GitError> {
        let mono_storage = self.storage.mono_storage();
        // check annotated DB first
        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(tag)) => return Ok(Some(self.tag_model_to_info(tag))),
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while getting tag: {}", e);
                return Err(GitError::CustomError("[code:500] DB error".to_string()));
            }
        }
        // check refs for lightweight tag
        let _repo_path = repo_path.unwrap_or_else(|| "/".to_string());
        let full_ref = format!("refs/tags/{}", name.clone());
        if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
            return Ok(Some(TagInfo {
                name: name.clone(),
                tag_id: r.ref_commit_hash.clone(),
                object_id: r.ref_commit_hash.clone(),
                object_type: "commit".to_string(),
                tagger: "".to_string(),
                message: "".to_string(),
                created_at: r.created_at.and_utc().to_rfc3339(),
            }));
        }
        Ok(None)
    }

    async fn delete_tag(&self, repo_path: Option<String>, name: String) -> Result<(), GitError> {
        let mono_storage = self.storage.mono_storage();
        // check annotated in DB first
        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(_tag)) => {
                // remove ref if exists
                let full_ref = format!("refs/tags/{}", name.clone());
                if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
                    mono_storage.remove_ref(r).await.map_err(|e| {
                        tracing::error!("Failed to remove ref while deleting annotated tag: {}", e);
                        GitError::CustomError("[code:500] Failed to remove ref".to_string())
                    })?;
                }
                mono_storage.delete_tag_by_name(&name).await.map_err(|e| {
                    tracing::error!("DB delete error when deleting annotated tag: {}", e);
                    GitError::CustomError("[code:500] DB delete error".to_string())
                })?;
                Ok(())
            }
            Ok(None) => {
                // try delete lightweight ref
                let _repo_path = repo_path.unwrap_or_else(|| "/".to_string());
                let full_ref = format!("refs/tags/{}", name.clone());
                // find ref by name and remove
                if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
                    mono_storage.remove_ref(r).await.map_err(|e| {
                        tracing::error!(
                            "Failed to remove ref while deleting lightweight tag: {}",
                            e
                        );
                        GitError::CustomError("[code:500] Failed to remove ref".to_string())
                    })?;
                    Ok(())
                } else {
                    Err(GitError::CustomError(
                        "[code:404] Tag not found".to_string(),
                    ))
                }
            }
            Err(e) => {
                tracing::error!("DB error while deleting tag: {}", e);
                Err(GitError::CustomError("[code:500] DB error".to_string()))
            }
        }
    }

    /// Get blame information for a file
    async fn get_file_blame(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        query: BlameQuery,
    ) -> Result<BlameResult, GitError> {
        tracing::info!(
            "Getting blame for file: {} at ref: {:?}",
            file_path,
            ref_name
        );

        // Validate input parameters
        if file_path.is_empty() {
            return Err(GitError::CustomError(
                "File path cannot be empty".to_string(),
            ));
        }

        // Use refs parameter if provided, otherwise use "main" as default
        let ref_name = if let Some(ref_name) = ref_name {
            if ref_name.is_empty() {
                "main"
            } else {
                ref_name
            }
        } else {
            "main"
        };

        // Use Jupiter's blame service
        let blame_service = BlameService::new(Arc::new(self.storage.clone()));

        // Convert API query to DTO query
        let dto_query: jupiter::model::blame_dto::BlameQuery = query.into();

        // 🔍 Step 1: Check if it is a large file
        let is_large_file = match blame_service
            .check_if_large_file(file_path, Some(ref_name))
            .await
        {
            Ok(is_large) => is_large,
            Err(e) => {
                tracing::warn!(
                    "Failed to check file size for {}: {}, using normal processing",
                    file_path,
                    e
                );
                false
            }
        };

        tracing::info!(
            "File {} is {} file, using {} processing",
            file_path,
            if is_large_file { "large" } else { "normal" },
            if is_large_file {
                "streaming"
            } else {
                "standard"
            }
        );

        // 🚀 Step 2: Select the processing method based on file size
        let blame_result = if is_large_file {
            // Large file: Use streaming processing
            tracing::info!("Using streaming processing for large file: {}", file_path);
            blame_service
                .get_file_blame_streaming_auto(file_path, Some(ref_name), dto_query)
                .await
        } else {
            // Normal file: Use standard processing
            tracing::info!("Using standard processing for normal file: {}", file_path);
            blame_service
                .get_file_blame(file_path, Some(ref_name), Some(dto_query))
                .await
        };

        match blame_result {
            Ok(result_from_service) => {
                // Convert DTO result to API result
                Ok(result_from_service.into())
            }
            Err(e) => {
                tracing::error!("Blame operation failed for {}: {}", file_path, e);
                Err(e)
            }
        }
    }
}

impl MonoApiService {
    // helper to convert mega_tag model into TagInfo
    fn tag_model_to_info(&self, tag: mega_tag::Model) -> TagInfo {
        TagInfo {
            name: tag.tag_name,
            tag_id: tag.tag_id,
            object_id: tag.object_id,
            object_type: tag.object_type,
            tagger: tag.tagger,
            message: tag.message,
            created_at: tag.created_at.and_utc().to_rfc3339(),
        }
    }

    async fn create_annotated_tag_mono(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_info: String,
        message: Option<String>,
        full_ref: String,
    ) -> Result<TagInfo, GitError> {
        let mono_storage = self.storage.mono_storage();

        // build git_internal/mega tag models
        let (tag_id_hex, object_id) = self.build_git_internal_tag_mono(
            name.clone(),
            target.clone(),
            tagger_info.clone(),
            message.clone(),
        )?;
        let tag_model = self.build_mega_tag_model(
            tag_id_hex.clone(),
            object_id.clone(),
            name.clone(),
            tagger_info.clone(),
            message.clone(),
        );

        match mono_storage.insert_tag(tag_model).await {
            Ok(saved_tag) => {
                // try to write ref; if ref write fails, rollback DB insert
                let path_str = repo_path.unwrap_or_else(|| "/".to_string());
                let tree_hash = common::utils::ZERO_ID.to_string();
                if let Err(e) = mono_storage
                    .save_ref(
                        &path_str,
                        Some(full_ref.clone()),
                        &object_id,
                        &tree_hash,
                        false,
                    )
                    .await
                {
                    // attempt to remove DB record
                    if let Err(del_e) = mono_storage.delete_tag_by_name(&name).await {
                        tracing::error!(
                            "Failed to rollback tag DB record after ref write failure: {}",
                            del_e
                        );
                    }
                    tracing::error!("Failed to write ref after DB insert: {}", e);
                    return Err(GitError::CustomError(
                        "[code:500] Failed to write ref".to_string(),
                    ));
                }
                Ok(self.tag_model_to_info(saved_tag))
            }
            Err(e) => {
                tracing::error!("DB insert error when creating annotated tag: {}", e);
                Err(GitError::CustomError(
                    "[code:500] DB insert error".to_string(),
                ))
            }
        }
    }

    async fn create_lightweight_tag_mono(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_info: String,
        full_ref: String,
    ) -> Result<TagInfo, GitError> {
        let mono_storage = self.storage.mono_storage();

        let path_str = repo_path.unwrap_or_else(|| "/".to_string());
        let object_id = target.clone().unwrap_or_default();
        let tree_hash = common::utils::ZERO_ID.to_string();
        mono_storage
            .save_ref(
                &path_str,
                Some(full_ref.clone()),
                &object_id,
                &tree_hash,
                false,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to write lightweight tag ref: {}", e);
                GitError::CustomError("[code:500] Failed to write lightweight tag ref".to_string())
            })?;
        // Fetch saved ref to use its creation time
        let saved_ref = mono_storage
            .get_ref_by_name(&full_ref)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?
            .ok_or_else(|| GitError::CustomError("Ref not found after creation".to_string()))?;

        Ok(TagInfo {
            name: name.clone(),
            tag_id: object_id.clone(),
            object_id: object_id.clone(),
            object_type: "commit".to_string(),
            tagger: tagger_info.clone(),
            message: String::new(),
            created_at: saved_ref.created_at.and_utc().to_rfc3339(),
        })
    }
    async fn validate_target_commit_mono(&self, target: Option<&String>) -> Result<(), GitError> {
        let mono_storage = self.storage.mono_storage();
        if let Some(ref t) = target {
            match mono_storage.get_commit_by_hash(t).await {
                Ok(commit_opt) => {
                    if commit_opt.is_none() {
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

    fn build_git_internal_tag_mono(
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
        let tagger_sig = git_internal::internal::object::signature::Signature::new(
            git_internal::internal::object::signature::SignatureType::Tagger,
            tagger_info.clone(),
            String::new(),
        );
        let git_internal_tag = git_internal::internal::object::tag::Tag::new(
            tag_target,
            git_internal::internal::object::types::ObjectType::Commit,
            name.clone(),
            tagger_sig,
            message.clone().unwrap_or_default(),
        );
        Ok((
            git_internal_tag.id.to_string(),
            target.unwrap_or_else(|| "HEAD".to_string()),
        ))
    }

    fn build_mega_tag_model(
        &self,
        tag_id_hex: String,
        object_id: String,
        name: String,
        tagger_info: String,
        message: Option<String>,
    ) -> mega_tag::Model {
        mega_tag::Model {
            id: common::utils::generate_id(),
            tag_id: tag_id_hex,
            object_id,
            object_type: "commit".to_string(),
            tag_name: name,
            tagger: tagger_info,
            message: message.unwrap_or_default(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
    pub async fn merge_cl(&self, username: &str, cl: mega_cl::Model) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();
        let refs = storage.get_ref(&cl.path).await.unwrap().unwrap();

        if cl.from_hash == refs.ref_commit_hash {
            let commit: Commit = Commit::from_mega_model(
                storage
                    .get_commit_by_hash(&cl.to_hash)
                    .await
                    .unwrap()
                    .unwrap(),
            );

            if cl.path != "/" {
                let path = PathBuf::from(cl.path.clone());
                // because only parent tree is needed so we skip current directory
                let update_chain = self.search_tree_for_update(path.parent().unwrap()).await?;
                let result = self.build_result_by_chain(path, update_chain, commit.tree_id)?;
                self.apply_update_result(&result, "cl merge generated commit")
                    .await?;
                // remove refs start with path except cl type
                storage.remove_none_cl_refs(&cl.path).await.unwrap();
                // TODO: self.clean_dangling_commits().await;
            }
            // add conversation
            self.storage
                .conversation_storage()
                .add_conversation(&cl.link, username, None, ConvTypeEnum::Merged)
                .await
                .unwrap();
            // update cl status last
            self.storage
                .cl_storage()
                .merge_cl(cl.clone())
                .await
                .unwrap();
        } else {
            return Err(GitError::CustomError("ref hash conflict".to_owned()));
        }
        Ok(())
    }

    /// Traverse parent trees and update them with the new commit's tree hash.
    /// This function only prepares updated trees and optionally a new parent commit.
    pub fn build_result_by_chain(
        &self,
        mut path: PathBuf,
        mut update_chain: Vec<Arc<Tree>>,
        mut updated_tree_hash: SHA1,
    ) -> Result<TreeUpdateResult, GitError> {
        let mut updated_trees = Vec::new();
        let mut ref_updates = Vec::new();

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

            let path_str = path.to_string_lossy().to_string();
            if path == Path::new("/") {
                ref_updates.push(RefUpdate::Update {
                    path: path_str,
                    tree_id: updated_tree_hash,
                });
            } else {
                ref_updates.push(RefUpdate::Delete { path: path_str });
            }
        }

        Ok(TreeUpdateResult {
            updated_trees,
            ref_updates,
        })
    }

    pub async fn apply_update_result(
        &self,
        result: &TreeUpdateResult,
        commit_msg: &str,
    ) -> Result<String, GitError> {
        let storage = self.storage.mono_storage();
        let mut new_commit_id = String::new();

        for update in &result.ref_updates {
            match update {
                RefUpdate::Update { path, tree_id } => {
                    // update can only be root path
                    if let Some(mut p_ref) = storage
                        .get_ref(path)
                        .await
                        .map_err(|e| GitError::CustomError(e.to_string()))?
                    {
                        let commit = Commit::from_tree_id(
                            *tree_id,
                            vec![SHA1::from_str(&p_ref.ref_commit_hash).unwrap()],
                            commit_msg,
                        );
                        new_commit_id = commit.id.to_string();
                        p_ref.ref_commit_hash = new_commit_id.clone();
                        p_ref.ref_tree_hash = tree_id.to_string();
                        storage
                            .update_ref(p_ref)
                            .await
                            .map_err(|e| GitError::CustomError(e.to_string()))?;
                        storage
                            .save_mega_commits(vec![commit])
                            .await
                            .map_err(|e| GitError::CustomError(e.to_string()))?;
                    }
                }
                RefUpdate::Delete { path } => {
                    if let Some(p_ref) = storage
                        .get_ref(path)
                        .await
                        .map_err(|e| GitError::CustomError(e.to_string()))?
                    {
                        storage
                            .remove_ref(p_ref)
                            .await
                            .map_err(|e| GitError::CustomError(e.to_string()))?;
                    }
                }
            }
        }

        let save_trees: Vec<mega_tree::ActiveModel> = result
            .updated_trees
            .clone()
            .into_iter()
            .map(|save_t| {
                let mut tree_model: mega_tree::Model = save_t.into_mega_model();
                tree_model.commit_id.clone_from(&new_commit_id);
                tree_model.into()
            })
            .collect();
        storage
            .batch_save_model(save_trees)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        Ok(new_commit_id)
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

    /// Fetches the content difference for a merge request, paginated by page_id and page_size.
    /// # Arguments
    /// * `cl_link` - The link to the merge request.
    /// * `page_id` - The page number to fetch. (id out of bounds will return empty)
    /// * `page_size` - The number of items per page.
    /// # Returns
    ///  a `Result` containing `ClDiff` on success or a `GitError` on failure.
    pub async fn paged_content_diff(
        &self,
        cl_link: &str,
        page: Pagination,
    ) -> Result<(Vec<DiffItem>, u64), GitError> {
        let per_page = page.per_page as usize;
        let page_id = page.page as usize;

        // old and new blobs for comparison
        let stg = self.storage.cl_storage();
        let cl =
            stg.get_cl(cl_link).await.unwrap().ok_or_else(|| {
                GitError::CustomError(format!("Merge request not found: {cl_link}"))
            })?;
        let old_blobs = self
            .get_commit_blobs(&cl.from_hash)
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get old commit blobs: {e}")))?;
        let new_blobs = self
            .get_commit_blobs(&cl.to_hash)
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get new commit blobs: {e}")))?;

        // calculate pages
        let sorted_changed_files = self
            .cl_files_list(old_blobs.clone(), new_blobs.clone())
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        // ensure page_id is within bounds
        let start = (page_id.saturating_sub(1)) * per_page;
        let end = (start + per_page).min(sorted_changed_files.len());

        let page_slice: &[ClDiffFile] = if start < sorted_changed_files.len() {
            let start_idx = start;
            let end_idx = end;
            &sorted_changed_files[start_idx..end_idx]
        } else {
            &[]
        };

        // create filtered files
        let mut page_old_blobs = Vec::new();
        let mut page_new_blobs = Vec::new();
        self.collect_page_blobs(page_slice, &mut page_old_blobs, &mut page_new_blobs);

        // get diff output
        let diff_output = self
            .get_diff_by_blobs(page_old_blobs, page_new_blobs)
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get diff output: {e}")))?;

        // calculate total pages
        let total = sorted_changed_files.len().div_ceil(per_page);

        Ok((diff_output, total as u64))
    }

    async fn get_diff_by_blobs(
        &self,
        old_blobs: Vec<(PathBuf, SHA1)>,
        new_blobs: Vec<(PathBuf, SHA1)>,
    ) -> Result<Vec<DiffItem>, GitError> {
        let mut blob_cache: HashMap<SHA1, Vec<u8>> = HashMap::new();

        // Collect all unique hashes
        let mut all_hashes = HashSet::new();
        for (_, hash) in &old_blobs {
            all_hashes.insert(*hash);
        }
        for (_, hash) in &new_blobs {
            all_hashes.insert(*hash);
        }

        // Fetch all blobs with better error handling and logging
        let mut failed_hashes = Vec::new();
        for hash in all_hashes {
            match self.get_raw_blob_by_hash(&hash.to_string()).await {
                Ok(Some(blob)) => {
                    blob_cache.insert(hash, blob.data.unwrap_or_default());
                }
                Ok(None) => {
                    tracing::warn!("Blob not found for hash: {}", hash);
                    blob_cache.insert(hash, Vec::new());
                }
                Err(e) => {
                    tracing::error!("Failed to fetch blob {}: {}", hash, e);
                    failed_hashes.push(hash);
                    blob_cache.insert(hash, Vec::new());
                }
            }
        }

        if !failed_hashes.is_empty() {
            tracing::warn!(
                "Failed to fetch {} blob(s): {:?}",
                failed_hashes.len(),
                failed_hashes
            );
        }

        // Enhanced content reader with better error handling
        let read_content = |file: &PathBuf, hash: &SHA1| -> Vec<u8> {
            match blob_cache.get(hash) {
                Some(content) => content.clone(),
                None => {
                    tracing::warn!("Missing blob content for file: {:?}, hash: {}", file, hash);
                    Vec::new()
                }
            }
        };

        // Use the unified diff function with configurable algorithm
        let diff_output = Diff::diff(
            old_blobs,
            new_blobs,
            "histogram".to_string(),
            Vec::new(),
            read_content,
        )
        .await;

        Ok(diff_output)
    }

    fn collect_page_blobs(
        &self,
        items: &[ClDiffFile],
        old_out: &mut Vec<(PathBuf, SHA1)>,
        new_out: &mut Vec<(PathBuf, SHA1)>,
    ) {
        old_out.reserve(items.len());
        new_out.reserve(items.len());

        for item in items {
            match item {
                ClDiffFile::New(p, h_new) => {
                    new_out.push((p.clone(), *h_new));
                }
                ClDiffFile::Deleted(p, h_old) => {
                    old_out.push((p.clone(), *h_old));
                }
                ClDiffFile::Modified(p, h_old, h_new) => {
                    old_out.push((p.clone(), *h_old));
                    new_out.push((p.clone(), *h_new));
                }
            }
        }
    }

    pub async fn get_sorted_changed_file_list(
        &self,
        cl_link: &str,
        path: Option<&str>,
    ) -> Result<Vec<String>, MegaError> {
        let cl = self
            .storage
            .cl_storage()
            .get_cl(cl_link)
            .await
            .unwrap()
            .ok_or_else(|| MegaError::with_message("Error getting "))?;

        let old_files = self.get_commit_blobs(&cl.from_hash.clone()).await?;
        let new_files = self.get_commit_blobs(&cl.to_hash.clone()).await?;

        // calculate pages
        let sorted_changed_files = self
            .cl_files_list(old_files.clone(), new_files.clone())
            .await?;
        let file_paths: Vec<String> = sorted_changed_files
            .iter()
            .map(|f| f.path().to_string_lossy().to_string())
            .filter(|file_path| {
                if let Some(prefix) = path {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .collect();

        Ok(file_paths)
    }

    pub async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, SHA1)>,
        new_files: Vec<(PathBuf, SHA1)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        let old_files: HashMap<PathBuf, SHA1> = old_files.into_iter().collect();
        let new_files: HashMap<PathBuf, SHA1> = new_files.into_iter().collect();
        let unions: HashSet<PathBuf> = old_files.keys().chain(new_files.keys()).cloned().collect();
        let mut res = vec![];
        for path in unions {
            let old_hash = old_files.get(&path);
            let new_hash = new_files.get(&path);
            match (old_hash, new_hash) {
                (None, None) => {}
                (None, Some(new)) => res.push(ClDiffFile::New(path, *new)),
                (Some(old), None) => res.push(ClDiffFile::Deleted(path, *old)),
                (Some(old), Some(new)) => {
                    if old == new {
                        continue;
                    } else {
                        res.push(ClDiffFile::Modified(path, *old, *new));
                    }
                }
            }
        }

        // Sort the results
        res.sort_by(|a, b| {
            a.path()
                .cmp(b.path())
                .then_with(|| a.kind_weight().cmp(&b.kind_weight()))
        });
        Ok(res)
    }

    pub async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, SHA1)>, MegaError> {
        let mut res = vec![];
        let mono_storage = self.storage.mono_storage();
        let commit = mono_storage.get_commit_by_hash(commit_hash).await?;
        if let Some(commit) = commit {
            let tree = mono_storage.get_tree_by_hash(&commit.tree).await?;
            if let Some(tree) = tree {
                let tree: Tree = Tree::from_mega_model(tree);
                res = self.traverse_tree(tree).await?;
            }
        }
        Ok(res)
    }

    pub async fn sync_third_party_repo(
        &self,
        owner: &str,
        repo: &str,
        mega_path: PathBuf,
    ) -> Result<Bytes, MegaError> {
        // Additional Path Parameter for Mega
        let url = format!("https://github.com/{owner}/{repo}.git");
        let remote_client = ThirdPartyClient::new(&url);

        let (ref_name, ref_hash) = remote_client.fetch_refs().await?;

        let res = remote_client
            .fetch_packs(std::slice::from_ref(&ref_hash))
            .await?;
        let pack_data = remote_client
            .process_pack_stream(res)
            .await
            .map_err(|e| MegaError::with_message(format!("{e}")))?;

        self.save_import_ref(&mega_path, &ref_name, &ref_hash)
            .await?;

        let shared = Arc::new(tokio::sync::Mutex::new(0));
        let mut protocol = SmartProtocol::new(
            mega_path,
            self.storage.clone(),
            shared,
            TransportProtocol::Http,
        );
        let bytes = protocol
            .git_receive_pack_stream(Box::pin(tokio_stream::once(Ok(Bytes::from(pack_data)))))
            .await
            .map_err(|e| MegaError::with_message(format!("{e}")))?;

        Ok(bytes)
    }

    async fn save_import_ref(
        &self,
        mega_path: &Path,
        ref_name: &str,
        ref_id: &str,
    ) -> Result<(), MegaError> {
        let path = mega_path
            .to_str()
            .ok_or_else(|| MegaError::with_message("Invalid UTF-8 in mega_path"))?;

        let name = mega_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MegaError::with_message("Failed to extract file name from mega_path"))?;

        self.storage
            .git_db_storage()
            .create_repo_and_save_ref(path, name, ref_name, ref_id)
            .await?;
        Ok(())
    }

    async fn traverse_tree(&self, root_tree: Tree) -> Result<Vec<(PathBuf, SHA1)>, MegaError> {
        let mut result = vec![];
        let mut stack = vec![(PathBuf::new(), root_tree)];

        while let Some((base_path, tree)) = stack.pop() {
            for item in tree.tree_items {
                let path = base_path.join(&item.name);
                if item.is_tree() {
                    let child = self
                        .storage
                        .mono_storage()
                        .get_tree_by_hash(&item.id.to_string())
                        .await?
                        .unwrap();
                    stack.push((path.clone(), Tree::from_mega_model(child)));
                } else {
                    result.push((path, item.id));
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::change_list::ClDiffFile;
    use git_internal::hash::SHA1;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    pub fn test_path() {
        let mut full_path = PathBuf::from("/project/rust/mega");
        for _ in 0..3 {
            let cloned_path = full_path.clone(); // Clone full_path
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            full_path.pop();
            println!("name: {name}, path: {full_path:?}");
        }
    }

    #[test]
    fn test_paging_calculation_basic() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
                SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                SHA1::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
    }

    #[test]
    fn test_paging_calculation_second_page() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
                SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                SHA1::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
            ClDiffFile::New(
                PathBuf::from("file4.txt"),
                SHA1::from_str("2222222222222222222222222222222222222222").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 2u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 2);
        assert_eq!(end, 4);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
        assert_eq!(page_slice[0].path(), &PathBuf::from("file3.txt"));
        assert_eq!(page_slice[1].path(), &PathBuf::from("file4.txt"));
    }

    #[test]
    fn test_paging_calculation_partial_page() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
                SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                SHA1::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
        ];

        let page_size = 5u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 3);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 3);
    }

    #[test]
    fn test_paging_calculation_out_of_bounds() {
        let files: Vec<ClDiffFile> = vec![ClDiffFile::New(
            PathBuf::from("file1.txt"),
            SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let page_size = 2u32;
        let page_id = 3u32; // Page that doesn't exist

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 4);
        assert_eq!(end, 1); // end is clamped to files.len()

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 0);
    }

    #[test]
    fn test_paging_calculation_edge_case_zero_page_size() {
        let files: Vec<ClDiffFile> = vec![ClDiffFile::New(
            PathBuf::from("file1.txt"),
            SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let page_size = 0u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 0);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 0);
    }

    #[test]
    fn test_paging_calculation_zero_page_id() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
                SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 0u32; // Should be treated as page 1 due to saturating_sub

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
    }

    #[test]
    fn test_paging_algorithm() {
        let total_files = 10usize;
        let current_page = 2u32;
        let page_size = 3u32;

        let total_pages = total_files.div_ceil(page_size as usize);
        let current_page = current_page as usize;
        let page_size = page_size as usize;

        assert_eq!(total_pages, 4);
        assert_eq!(current_page, 2);
        assert_eq!(page_size, 3);
    }

    #[test]
    fn test_collect_page_blobs_new_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![ClDiffFile::New(
            PathBuf::from("new_file.txt"),
            SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        service.collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 0);
        assert_eq!(new_blobs.len(), 1);
        assert_eq!(new_blobs[0].0, PathBuf::from("new_file.txt"));
    }

    #[test]
    fn test_collect_page_blobs_deleted_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![ClDiffFile::Deleted(
            PathBuf::from("deleted_file.txt"),
            SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        service.collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 1);
        assert_eq!(new_blobs.len(), 0);
        assert_eq!(old_blobs[0].0, PathBuf::from("deleted_file.txt"));
    }

    #[test]
    fn test_file_lists_with_roots() {
        let all_files = vec![
            "src/main.rs".to_string(),
            "src/utils/math.rs".to_string(),
            "src/utils/io.rs".to_string(),
            "README.md".to_string(),
        ];

        let root: Option<&str> = None;
        let filtered_none: Vec<String> = all_files
            .iter()
            .filter(|file_path| {
                if let Some(prefix) = root {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        assert_eq!(filtered_none.len(), 4);
        assert_eq!(filtered_none, all_files);

        let filtered_some: Vec<String> = all_files
            .iter()
            .filter(|file_path| {
                if let Some(prefix) = Some("src/utils") {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        assert_eq!(filtered_some.len(), 2);
        assert_eq!(
            filtered_some,
            vec![
                "src/utils/math.rs".to_string(),
                "src/utils/io.rs".to_string()
            ]
        );
    }

    #[test]
    fn test_collect_page_blobs_modified_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![ClDiffFile::Modified(
            PathBuf::from("modified_file.txt"),
            SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
            SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        service.collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 1);
        assert_eq!(new_blobs.len(), 1);
        assert_eq!(old_blobs[0].0, PathBuf::from("modified_file.txt"));
        assert_eq!(new_blobs[0].0, PathBuf::from("modified_file.txt"));
    }

    #[test]
    fn test_collect_page_blobs_mixed_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![
            ClDiffFile::New(
                PathBuf::from("new.txt"),
                SHA1::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("deleted.txt"),
                SHA1::from_str("2222222222222222222222222222222222222222").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("modified.txt"),
                SHA1::from_str("3333333333333333333333333333333333333333").unwrap(),
                SHA1::from_str("4444444444444444444444444444444444444444").unwrap(),
            ),
        ];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        service.collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 2); // deleted + modified
        assert_eq!(new_blobs.len(), 2); // new + modified

        assert_eq!(old_blobs[0].0, PathBuf::from("deleted.txt"));
        assert_eq!(old_blobs[1].0, PathBuf::from("modified.txt"));
        assert_eq!(new_blobs[0].0, PathBuf::from("new.txt"));
        assert_eq!(new_blobs[1].0, PathBuf::from("modified.txt"));
    }

    #[tokio::test]
    async fn test_content_diff_functionality() {
        use git_internal::internal::object::blob::Blob;
        use std::collections::HashMap;

        // Test basic diff generation with sample data
        let old_content = "Hello World\nLine 2\nLine 3";
        let new_content = "Hello Universe\nLine 2\nLine 3 modified";

        let old_blob = Blob::from_content(old_content);
        let new_blob = Blob::from_content(new_content);

        let old_blobs = vec![(PathBuf::from("test_file.txt"), old_blob.id)];
        let new_blobs = vec![(PathBuf::from("test_file.txt"), new_blob.id)];

        // Create a blob cache for the test
        let mut blob_cache: HashMap<SHA1, Vec<u8>> = HashMap::new();
        blob_cache.insert(old_blob.id, old_content.as_bytes().to_vec());
        blob_cache.insert(new_blob.id, new_content.as_bytes().to_vec());

        // Test the diff engine directly
        let read_content = |_file: &PathBuf, hash: &SHA1| -> Vec<u8> {
            blob_cache.get(hash).cloned().unwrap_or_default()
        };

        let diff_output = Diff::diff(
            old_blobs,
            new_blobs,
            "histogram".to_string(),
            Vec::new(),
            read_content,
        )
        .await;

        // Verify diff output contains expected content
        assert!(!diff_output.is_empty(), "Diff output should not be empty");
        assert_eq!(diff_output.len(), 1, "Should have diff for one file");

        let diff_item = &diff_output[0];
        assert_eq!(diff_item.path, "test_file.txt");
        assert!(
            diff_item.data.contains("diff --git"),
            "Should contain git diff header"
        );
        assert!(
            diff_item.data.contains("-Hello World"),
            "Should show removed line"
        );
        assert!(
            diff_item.data.contains("+Hello Universe"),
            "Should show added line"
        );
        assert!(diff_item.data.contains("-Line 3"), "Should show old line 3");
        assert!(
            diff_item.data.contains("+Line 3 modified"),
            "Should show new line 3"
        );
    }

    #[tokio::test]
    async fn test_get_diff_by_blobs_with_empty_content() {
        // Test diff generation with empty content (simulating missing blobs)
        let old_hash = SHA1::from_str("1234567890123456789012345678901234567890").unwrap();
        let new_hash = SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap();

        let old_blobs = vec![(PathBuf::from("empty_file.txt"), old_hash)];
        let new_blobs = vec![(PathBuf::from("empty_file.txt"), new_hash)];

        // Create empty blob cache to simulate missing blobs
        let blob_cache: HashMap<SHA1, Vec<u8>> = HashMap::new();

        let read_content = |_file: &PathBuf, hash: &SHA1| -> Vec<u8> {
            blob_cache.get(hash).cloned().unwrap_or_default()
        };

        // Test the diff engine with empty content
        let diff_output = Diff::diff(
            old_blobs,
            new_blobs,
            "histogram".to_string(),
            Vec::new(),
            read_content,
        )
        .await;

        assert!(
            !diff_output.is_empty(),
            "Should generate diff even with empty blobs"
        );
        assert_eq!(diff_output[0].path, "empty_file.txt");
        assert!(
            diff_output[0].data.contains("diff --git"),
            "Should contain git diff header"
        );
    }
}

#[test]
fn test_parse_github_link() {
    let url = "https://github.com/web3infra-foundation/libra/";
    let url = url
        .trim_end_matches(".git")
        .trim_end_matches("/")
        .strip_prefix("https://github.com/")
        .expect("Invalid GitHub URL");
    let (owner, repo) = url.rsplit_once('/').unwrap();
    assert_eq!(owner, "web3infra-foundation");
    assert_eq!(repo, "libra");
}

#[tokio::test]
async fn test_third_party_trait() {
    let url = "https://github.com/aidcheng/mega.git";
    let third_party_client = ThirdPartyClient::new(url);

    let (_, refs) = third_party_client
        .fetch_refs()
        .await
        .expect("Unable to fetch refs");

    let res = third_party_client
        .fetch_packs(&[refs])
        .await
        .expect("Unable to fetch res");

    third_party_client
        .process_pack_stream(res)
        .await
        .expect("unable to process");
}
