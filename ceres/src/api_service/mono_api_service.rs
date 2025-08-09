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
//! - `mercury`: Git object handling and version control primitives
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

use async_trait::async_trait;

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_blob, mega_mr, mega_tree, raw_blob};
use common::errors::MegaError;
use neptune::neptune_engine::Diff;
use jupiter::storage::base_storage::StorageConnector;
use jupiter::storage::Storage;
use jupiter::utils::converter::generate_git_keep_with_timestamp;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};

use crate::api_service::ApiHandler;
use crate::model::git::CreateFileInfo;
use crate::model::mr::{MrDiffFile, MrDiff, MrPageInfo};


#[derive(Clone)]
pub struct MonoApiService {
    pub storage: Storage,
}

#[async_trait]
impl ApiHandler for MonoApiService {
    fn get_context(&self) -> Storage {
        self.storage.clone()
    }

    /// Creates a new file or directory in the monorepo based on the provided file information.
    ///
    /// # Arguments
    ///
    /// * `file_info` - Information about the file or directory to create.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `GitError` on failure.
    async fn create_monorepo_file(&self, file_info: CreateFileInfo) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();
        let path = PathBuf::from(file_info.path);
        let mut save_trees = vec![];

        // Search for the tree to update and get its tree items
        let (update_trees, search_tree) = self.search_tree_for_update(&path).await?;
        let mut t_items = search_tree.tree_items;

        // Create a new tree item based on whether it's a directory or file
        let new_item = if file_info.is_directory {
            if t_items
                .iter()
                .any(|x| x.mode == TreeItemMode::Tree && x.name == file_info.name)
            {
                return Err(GitError::CustomError("Duplicate name".to_string()));
            }
            let blob = generate_git_keep_with_timestamp();
            let tree_item = TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: String::from(".gitkeep"),
            };
            let child_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
            save_trees.push(child_tree.clone());
            TreeItem {
                mode: TreeItemMode::Tree,
                id: child_tree.id,
                name: file_info.name.clone(),
            }
        } else {
            let content = file_info.content.unwrap();
            let blob = Blob::from_content(&content);
            let mega_blob: mega_blob::ActiveModel = Into::<mega_blob::Model>::into(&blob).into();
            let raw_blob: raw_blob::ActiveModel =
                Into::<raw_blob::Model>::into(blob.clone()).into();

            storage.batch_save_model(vec![mega_blob]).await.unwrap();
            storage.batch_save_model(vec![raw_blob]).await.unwrap();
            TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: file_info.name.clone(),
            }
        };
        // Add the new item to the tree items and create a new tree
        t_items.push(new_item);
        let p_tree = Tree::from_tree_items(t_items).unwrap();

        // Create a commit for the new tree
        let refs = storage.get_ref("/").await.unwrap().unwrap();
        let commit = Commit::from_tree_id(
            p_tree.id,
            vec![SHA1::from_str(&refs.ref_commit_hash).unwrap()],
            &format!("\ncreate file {} commit", file_info.name),
        );

        // Update the parent tree with the new commit
        let commit_id = self.update_parent_tree(path, update_trees, commit).await?;
        save_trees.push(p_tree);

        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|save_t| {
                let mut tree_model: mega_tree::Model = save_t.into();
                tree_model.commit_id.clone_from(&commit_id);
                tree_model.into()
            })
            .collect();
        storage.batch_save_model(save_trees).await.unwrap();

        Ok(())
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError> {
        Ok(path.to_path_buf())
    }

    async fn get_root_commit(&self) -> Commit {
        unreachable!()
    }

    async fn get_root_tree(&self) -> Tree {
        let storage = self.storage.mono_storage();
        let refs = storage.get_ref("/").await.unwrap().unwrap();

        storage
            .get_tree_by_hash(&refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        self.storage
            .mono_storage()
            .get_tree_by_hash(hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        match self.storage.mono_storage().get_commit_by_hash(hash).await {
            Ok(Some(commit)) => Some(commit.into()),
            _ => None,
        }
    }

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit {
        let storage = self.storage.mono_storage();
        let tree_info = storage.get_tree_by_hash(t_hash).await.unwrap().unwrap();
        storage
            .get_commit_by_hash(&tree_info.commit_id)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let commits = self
            .storage
            .mono_storage()
            .get_commits_by_hashes(&c_hashes)
            .await
            .unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
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
}

impl MonoApiService {
    pub async fn merge_mr(&self, username: &str, mr: mega_mr::Model) -> Result<(), MegaError> {
        let storage = self.storage.mono_storage();
        let refs = storage.get_ref(&mr.path).await.unwrap().unwrap();

        if mr.from_hash == refs.ref_commit_hash {
            let commit: Commit = storage
                .get_commit_by_hash(&mr.to_hash)
                .await
                .unwrap()
                .unwrap()
                .into();

            if mr.path != "/" {
                let path = PathBuf::from(mr.path.clone());
                // because only parent tree is needed so we skip current directory
                let (tree_vec, _) = self
                    .search_tree_for_update(path.parent().unwrap())
                    .await
                    .unwrap();
                self.update_parent_tree(path, tree_vec, commit)
                    .await
                    .unwrap();
                // remove refs start with path except mr type
                storage.remove_none_mr_refs(&mr.path).await.unwrap();
                // TODO: self.clean_dangling_commits().await;
            }
            // add conversation
            self.storage
                .conversation_storage()
                .add_conversation(&mr.link, username, None, ConvTypeEnum::Merged)
                .await
                .unwrap();
            // update mr status last
            self.storage
                .mr_storage()
                .merge_mr(mr.clone())
                .await
                .unwrap();
        } else {
            return Err(MegaError::with_message("ref hash conflict"));
        }
        Ok(())
    }

    async fn update_parent_tree(
        &self,
        mut path: PathBuf,
        mut tree_vec: Vec<Tree>,
        commit: Commit,
    ) -> Result<String, GitError> {
        let storage = self.storage.mono_storage();
        let mut save_trees = Vec::new();
        let mut p_commit_id = String::new();

        let mut target_hash = commit.tree_id;

        while let Some(mut tree) = tree_vec.pop() {
            let cloned_path = path.clone();
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            path.pop();

            let index = tree.tree_items.iter().position(|x| x.name == name).unwrap();
            tree.tree_items[index].id = target_hash;
            let new_tree = Tree::from_tree_items(tree.tree_items).unwrap();
            target_hash = new_tree.id;

            let model: mega_tree::Model = new_tree.into();
            save_trees.push(model);

            let p_ref = storage.get_ref(path.to_str().unwrap()).await.unwrap();
            if let Some(mut p_ref) = p_ref {
                if path == Path::new("/") {
                    let p_commit = Commit::new(
                        commit.author.clone(),
                        commit.committer.clone(),
                        target_hash,
                        vec![SHA1::from_str(&p_ref.ref_commit_hash).unwrap()],
                        &commit.message,
                    );
                    p_commit_id = p_commit.id.to_string();
                    // update p_ref
                    p_ref.ref_commit_hash = p_commit.id.to_string();
                    p_ref.ref_tree_hash = target_hash.to_string();
                    storage.update_ref(p_ref).await.unwrap();
                    storage.save_mega_commits(vec![p_commit]).await.unwrap();
                } else {
                    storage.remove_ref(p_ref).await.unwrap();
                }
            }
        }
        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|mut x| {
                p_commit_id.clone_into(&mut x.commit_id);
                x.into()
            })
            .collect();

        storage.batch_save_model(save_trees).await.unwrap();
        Ok(p_commit_id)
    }

    /// Fetches the content difference for a merge request, paginated by page_id and page_size.
    /// # Arguments
    /// * `mr_link` - The link to the merge request.
    /// * `page_id` - The page number to fetch. (id out of bounds will return empty)
    /// * `page_size` - The number of items per page.
    /// # Returns
    ///  a `Result` containing `MrDiff` on success or a `GitError` on failure.
    pub async fn content_diff(
        &self,
        mr_link: &str,
        page_id: usize,
        page_size: usize,
    ) -> Result<MrDiff, GitError> {
        // old and new blobs for comparison
        let stg = self.storage.mr_storage();
        let mr = stg.get_mr(mr_link).await.unwrap().ok_or_else(|| {
            GitError::CustomError(format!("Merge request not found: {mr_link}"))
        })?;
        let old_blobs = self.get_commit_blobs(&mr.from_hash).await.map_err(|e| {
            GitError::CustomError(format!("Failed to get old commit blobs: {e}"))
        })?;
        let new_blobs = self.get_commit_blobs(&mr.to_hash).await.map_err(|e| {
            GitError::CustomError(format!("Failed to get new commit blobs: {e}"))
        })?;

        // calculate pages
        let sorted_changed_files = self.mr_files_list(old_blobs.clone(), new_blobs.clone()).await?;

        // ensure page_id is within bounds
        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(sorted_changed_files.len());

        let page_slice: &[MrDiffFile] = if start < sorted_changed_files.len() {
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

        let mut blob_cache: HashMap<SHA1, Vec<u8>> = HashMap::new();

        // Collect all unique hashes
        let mut all_hashes = HashSet::new();
        for (_, hash) in &old_blobs {
            all_hashes.insert(*hash);
        }
        for (_, hash) in &new_blobs {
            all_hashes.insert(*hash);
        }

        // Fetch all blobs concurrently
        for hash in all_hashes {
            match self.get_raw_blob_by_hash(&hash.to_string()).await {
                Ok(Some(blob)) => {
                    blob_cache.insert(hash, blob.data.unwrap_or_default());
                }
                _ => {
                    blob_cache.insert(hash, Vec::new());
                }
            }
        }

        // Simple synchronous closure that uses the pre-fetched cache
        let read_content = |_file: &PathBuf, hash: &SHA1| -> Vec<u8>{
            blob_cache.get(hash).cloned().unwrap_or_default()
        };

        // Use the unified diff function that returns a single string
        let diff_output = Diff::diff(
            page_old_blobs,
            page_new_blobs,
            "histogram".to_string(),
            Vec::new(),
            read_content,
        ).await;

        Ok(MrDiff {
            data: diff_output,
            page_info: Some(MrPageInfo {
                total_pages: (sorted_changed_files.len()-1).div_ceil(page_size),
                current_page: page_id,
                page_size,
            })
        })
    }

    fn collect_page_blobs(
        &self,
        items: &[MrDiffFile],
        old_out: &mut Vec<(PathBuf, SHA1)>,
        new_out: &mut Vec<(PathBuf, SHA1)>,
    ) {
        old_out.reserve(items.len());
        new_out.reserve(items.len());

        for item in items {
            match item {
                MrDiffFile::New(p, h_new) => {
                    new_out.push((p.clone(), *h_new));
                }
                MrDiffFile::Deleted(p, h_old) => {
                    old_out.push((p.clone(), *h_old));
                }
                MrDiffFile::Modified(p, h_old, h_new) => {
                    old_out.push((p.clone(), *h_old));
                    new_out.push((p.clone(), *h_new));
                }
            }
        }
    }

    pub async fn mr_files_list(
        &self,
        old_files: Vec<(PathBuf, SHA1)>,
        new_files: Vec<(PathBuf, SHA1)>,
    ) -> Result<Vec<MrDiffFile>, MegaError> {
        let old_files: HashMap<PathBuf, SHA1> = old_files.into_iter().collect();
        let new_files: HashMap<PathBuf, SHA1> = new_files.into_iter().collect();
        let unions: HashSet<PathBuf> = old_files.keys().chain(new_files.keys()).cloned().collect();
        let mut res = vec![];
        for path in unions {
            let old_hash = old_files.get(&path);
            let new_hash = new_files.get(&path);
            match (old_hash, new_hash) {
                (None, None) => {}
                (None, Some(new)) => res.push(MrDiffFile::New(path, *new)),
                (Some(old), None) => res.push(MrDiffFile::Deleted(path, *old)),
                (Some(old), Some(new)) => {
                    if old == new {
                        continue;
                    } else {
                        res.push(MrDiffFile::Modified(path, *old, *new));
                    }
                }
            }
        }

        // Sort the results
        res.sort_by(|a, b| {
            a.path().cmp(b.path()).then_with(|| a.kind_weight().cmp(&b.kind_weight()))
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
                let tree: Tree = tree.into();
                res = self.traverse_tree(tree).await?;
            }
        }
        Ok(res)
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
                    stack.push((path.clone(), child.into()));
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
    use std::path::PathBuf;
    use mercury::hash::SHA1;
    use std::str::FromStr;
    use crate::model::mr::{MrDiffFile, MrPageInfo};

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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
            MrDiffFile::Modified(PathBuf::from("file2.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap(), SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()),
            MrDiffFile::Deleted(PathBuf::from("file3.txt"), SHA1::from_str("1111111111111111111111111111111111111111").unwrap()),
        ];

        let page_size = 2u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
            MrDiffFile::Modified(PathBuf::from("file2.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap(), SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()),
            MrDiffFile::Deleted(PathBuf::from("file3.txt"), SHA1::from_str("1111111111111111111111111111111111111111").unwrap()),
            MrDiffFile::New(PathBuf::from("file4.txt"), SHA1::from_str("2222222222222222222222222222222222222222").unwrap()),
        ];

        let page_size = 2u32;
        let page_id = 2u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 2);
        assert_eq!(end, 4);

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
            MrDiffFile::Modified(PathBuf::from("file2.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap(), SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()),
            MrDiffFile::Deleted(PathBuf::from("file3.txt"), SHA1::from_str("1111111111111111111111111111111111111111").unwrap()),
        ];

        let page_size = 5u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 3);

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
        ];

        let page_size = 2u32;
        let page_id = 3u32; // Page that doesn't exist

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 4);
        assert_eq!(end, 1); // end is clamped to files.len()

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
        ];

        let page_size = 0u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 0);

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
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
        let files: Vec<MrDiffFile> = vec![
            MrDiffFile::New(PathBuf::from("file1.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
            MrDiffFile::Modified(PathBuf::from("file2.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap(), SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()),
        ];

        let page_size = 2u32;
        let page_id = 0u32; // Should be treated as page 1 due to saturating_sub

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[MrDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
    }

    #[test]
    fn test_paging_page_info_construction() {
        let total_files = 10usize;
        let current_page = 2u32;
        let page_size = 3u32;

        let page_info = MrPageInfo {
            total_pages: (total_files + page_size as usize - 1) / page_size as usize,
            current_page: current_page as usize,
            page_size: page_size as usize,
        };

        assert_eq!(page_info.total_pages, 4);
        assert_eq!(page_info.current_page, 2);
        assert_eq!(page_info.page_size, 3);
    }

    #[test]
    fn test_collect_page_blobs_new_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![
            MrDiffFile::New(PathBuf::from("new_file.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
        ];

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

        let files = vec![
            MrDiffFile::Deleted(PathBuf::from("deleted_file.txt"), SHA1::from_str("1234567890123456789012345678901234567890").unwrap()),
        ];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        service.collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 1);
        assert_eq!(new_blobs.len(), 0);
        assert_eq!(old_blobs[0].0, PathBuf::from("deleted_file.txt"));
    }

    #[test]
    fn test_collect_page_blobs_modified_files() {
        let service = MonoApiService {
            storage: Storage::mock(),
        };

        let files = vec![
            MrDiffFile::Modified(
                PathBuf::from("modified_file.txt"),
                SHA1::from_str("1234567890123456789012345678901234567890").unwrap(),
                SHA1::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()
            ),
        ];

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
            MrDiffFile::New(PathBuf::from("new.txt"), SHA1::from_str("1111111111111111111111111111111111111111").unwrap()),
            MrDiffFile::Deleted(PathBuf::from("deleted.txt"), SHA1::from_str("2222222222222222222222222222222222222222").unwrap()),
            MrDiffFile::Modified(
                PathBuf::from("modified.txt"),
                SHA1::from_str("3333333333333333333333333333333333333333").unwrap(),
                SHA1::from_str("4444444444444444444444444444444444444444").unwrap()
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
}
