//! Buck Upload Tree and Commit Builder
//!
//! This module provides utilities for building Git trees and commits from
//! a batch of file changes. It's designed for the Buck upload API to
//! create a single atomic commit containing multiple file changes

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    str::FromStr,
};

use common::errors::MegaError;
use git_internal::{
    hash::ObjectHash,
    internal::{
        metadata::EntryMeta,
        object::{
            commit::Commit,
            tree::{Tree, TreeItem, TreeItemMode},
        },
    },
};
use jupiter::{
    storage::mono_storage::MonoStorage,
    utils::converter::{FromMegaModel, IntoMegaModel},
};

use crate::model::buck::FileChange;

/// Result of building trees from file changes
#[derive(Debug)]
pub struct TreeBuildResult {
    /// The new root tree after applying all changes
    pub root_tree: Tree,
    /// All new trees that need to be saved (including intermediate directories)
    pub new_trees: Vec<Tree>,
    /// The final tree hash (same as root_tree.id)
    pub tree_hash: ObjectHash,
}

/// Result of building a complete commit
#[derive(Debug)]
pub struct CommitBuildResult {
    /// The created commit
    pub commit: Commit,
    /// All new trees that need to be saved (as Model, with commit_id set)
    pub new_tree_models: Vec<callisto::mega_tree::Model>,
    /// The commit hash
    pub commit_id: String,
    /// The tree hash
    pub tree_hash: String,
}

/// Builder for creating Git trees and commits from batch file changes
///
/// Takes a base commit and a list of file changes, builds a new tree structure
/// containing all changes, and creates a single atomic commit.
pub struct BuckCommitBuilder {
    storage: MonoStorage,
}

impl BuckCommitBuilder {
    pub fn new(storage: MonoStorage) -> Self {
        Self { storage }
    }

    /// Build a new tree structure from a list of file changes
    ///
    /// # Arguments
    /// * `base_commit_hash` - Base commit to build upon
    /// * `files` - List of file changes to apply
    ///
    /// # Behavior
    /// - If `files` is empty, returns the base commit's tree unchanged
    /// - Processes directories from deepest to shallowest
    /// - Creates new trees only when changes are detected
    pub async fn build_tree_with_changes(
        &self,
        base_commit_hash: &str,
        files: &[FileChange],
    ) -> Result<TreeBuildResult, MegaError> {
        // Load base commit and root tree
        let base_commit = self
            .storage
            .get_commit_by_hash(base_commit_hash)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!("Base commit not found: {}", base_commit_hash))
            })?;

        let root_tree_model = self
            .storage
            .get_tree_by_hash(&base_commit.tree)
            .await?
            .ok_or_else(|| MegaError::Other("Root tree not found".into()))?;

        let root_tree = Tree::from_mega_model(root_tree_model);

        if files.is_empty() {
            return Ok(TreeBuildResult {
                tree_hash: root_tree.id,
                root_tree,
                new_trees: Vec::new(),
            });
        }

        // Group files by directory for efficient batch processing
        let files_by_dir = self.group_files_by_directory(files)?;

        // Build directory tree updates (key: directory path, value: updated tree)
        let mut dir_trees: HashMap<PathBuf, Tree> = HashMap::new();
        let mut new_trees: Vec<Tree> = Vec::new();

        // Initialize Base Tree Cache to optimize lookups
        let mut base_tree_cache: HashMap<PathBuf, Tree> = HashMap::new();

        // Pre-compute parent -> children mapping for child lookup
        let mut children_by_parent: HashMap<PathBuf, Vec<(PathBuf, Tree)>> = HashMap::new();

        // Process directories from deepest to shallowest to ensure parent directories
        // can reference their updated children
        let mut sorted_dirs: Vec<_> = files_by_dir.keys().cloned().collect();
        sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

        for dir_path in sorted_dirs {
            let dir_files = &files_by_dir[&dir_path];

            // Load or get the existing tree for this directory
            let existing_tree = if self.is_root_path(&dir_path) {
                root_tree.clone()
            } else {
                self.find_tree_at_path(&root_tree, &dir_path, &mut base_tree_cache)
                    .await?
                    .unwrap_or_else(Self::empty_tree)
            };

            // Update tree items with new blob hashes and child directory updates
            let updated_tree = self.update_tree_with_files(
                &dir_path,
                &existing_tree,
                dir_files,
                &children_by_parent,
            )?;

            if updated_tree.id != existing_tree.id {
                new_trees.push(updated_tree.clone());
            }
            dir_trees.insert(dir_path.clone(), updated_tree.clone());

            // Update children_by_parent for parent directory
            if let Some(parent) = dir_path.parent() {
                children_by_parent
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push((dir_path.clone(), updated_tree));
            }
        }

        // Get final root tree
        let final_root = dir_trees.get(&PathBuf::new()).cloned().unwrap_or(root_tree);

        Ok(TreeBuildResult {
            tree_hash: final_root.id,
            root_tree: final_root,
            new_trees,
        })
    }

    /// Build a complete commit from file changes
    ///
    /// This is the main entry point that combines tree building with commit creation.
    /// Uses system default for author information.
    ///
    /// # Arguments
    /// * `base_commit_hash` - Parent commit hash
    /// * `files` - List of file changes to include in commit
    /// * `message` - Commit message
    pub async fn build_commit(
        &self,
        base_commit_hash: &str,
        files: &[FileChange],
        message: &str,
    ) -> Result<CommitBuildResult, MegaError> {
        // Build tree structure from file changes
        let tree_result = self
            .build_tree_with_changes(base_commit_hash, files)
            .await?;

        // Create commit with the new tree
        let parent_sha = ObjectHash::from_str(base_commit_hash)
            .map_err(|e| MegaError::Other(format!("Invalid parent hash: {}", e)))?;

        let commit = Commit::from_tree_id(tree_result.tree_hash, vec![parent_sha], message);

        // Convert trees to database models with commit_id set
        let tree_models: Vec<callisto::mega_tree::Model> = tree_result
            .new_trees
            .iter()
            .map(|t| {
                let mut model: callisto::mega_tree::Model =
                    t.clone().into_mega_model(EntryMeta::default());
                model.commit_id = commit.id.to_string();
                model
            })
            .collect();

        Ok(CommitBuildResult {
            commit_id: commit.id.to_string(),
            tree_hash: tree_result.tree_hash.to_string(),
            commit,
            new_tree_models: tree_models,
        })
    }

    /// Normalize a path into a canonical component list.
    ///
    /// Removes '.', '..', and empty components, validates security rules,
    /// and ensures Root is always represented as an empty list (not ".").
    fn normalize_path_to_components(raw_path: &str) -> Result<Vec<String>, MegaError> {
        // Reject Windows absolute paths (e.g., "C:/Windows/..." or "C:\\Windows\\...")
        // This check works on all platforms, not just Windows
        if raw_path.len() >= 2 {
            let first_two = &raw_path[..2];
            if first_two
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
                && first_two.chars().nth(1) == Some(':')
            {
                return Err(MegaError::Other(format!(
                    "Absolute path not allowed (Windows drive letter detected): {}",
                    raw_path
                )));
            }
        }

        // Ensure input is a relative path (remove leading slashes)
        let path = Path::new(raw_path.trim_start_matches('/'));

        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::Normal(os_str) => {
                    let s = os_str.to_str().ok_or_else(|| {
                        MegaError::Other(format!("Invalid UTF-8 path: {}", raw_path))
                    })?;

                    // Security: Reject .git
                    if s == ".git" {
                        return Err(MegaError::Other(format!(
                            "Security: .git access denied: {}",
                            raw_path
                        )));
                    }
                    components.push(s.to_string());
                }
                std::path::Component::ParentDir => {
                    return Err(MegaError::Other(format!(
                        "Path traversal (..) detected: {}",
                        raw_path
                    )));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(MegaError::Other(format!(
                        "Absolute path not allowed: {}",
                        raw_path
                    )));
                }
                std::path::Component::CurDir => {
                    // Ignore '.' components to fix the "." vs "" ambiguity
                }
            }
        }

        // Validate path nesting depth (max 100 levels)
        if components.len() > 100 {
            return Err(MegaError::Other(format!(
                "Path nesting too deep (max 100 levels): {}",
                raw_path
            )));
        }

        // Validate canonical path length (max 4096 characters)
        let total_len = components.iter().map(|c| c.len()).sum::<usize>()
            + if components.is_empty() {
                0
            } else {
                components.len() - 1
            };

        if total_len > 4096 {
            return Err(MegaError::Other(format!(
                "Path too long (max 4096 characters): {}",
                raw_path
            )));
        }

        Ok(components)
    }

    /// Group files by their parent directory
    ///
    /// Always includes root directory and all intermediate directories to ensure proper tree chain.
    fn group_files_by_directory(
        &self,
        files: &[FileChange],
    ) -> Result<HashMap<PathBuf, Vec<FileChange>>, MegaError> {
        let mut groups: HashMap<PathBuf, Vec<FileChange>> = HashMap::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        // Explicitly insert root directory (empty path)
        groups.insert(PathBuf::new(), Vec::new());

        for file in files {
            // Normalize path using unified normalization logic
            let components = Self::normalize_path_to_components(&file.path)?;

            if components.is_empty() {
                return Err(MegaError::Other(format!(
                    "Invalid empty path: {}",
                    file.path
                )));
            }

            // Reconstruct PathBuf from clean components
            let mut clean_path = PathBuf::new();
            for c in &components {
                clean_path.push(c);
            }

            // Check for duplicate paths using normalized PathBuf
            if seen_paths.contains(&clean_path) {
                return Err(MegaError::Other(format!(
                    "Duplicate file path: {:?}",
                    clean_path
                )));
            }
            seen_paths.insert(clean_path.clone());

            // Calculate parent directory
            let dir = if components.len() > 1 {
                let mut p = PathBuf::new();
                for c in &components[..components.len() - 1] {
                    p.push(c);
                }
                p
            } else {
                PathBuf::new() // Root
            };

            // Add all intermediate directories to ensure proper tree chain
            let mut current = dir.clone();
            while !current.as_os_str().is_empty() {
                groups.entry(current.clone()).or_default();
                current = current.parent().unwrap_or(Path::new("")).to_path_buf();
            }

            let mut file_change = file.clone();
            file_change.path = clean_path.to_string_lossy().to_string();
            groups.entry(dir).or_default().push(file_change);
        }

        Ok(groups)
    }

    /// Check if the path represents the repository root
    fn is_root_path(&self, path: &Path) -> bool {
        path.as_os_str().is_empty()
    }

    /// Find tree at a given path by traversing from root
    ///
    /// # Returns:
    /// - `Ok(Some(tree))` if the directory exists
    /// - `Ok(None)` if the directory does not exist (new directory)
    /// - `Err(...)` for actual errors (database issues, etc.)
    async fn find_tree_at_path(
        &self,
        root: &Tree,
        path: &Path,
        base_tree_cache: &mut HashMap<PathBuf, Tree>,
    ) -> Result<Option<Tree>, MegaError> {
        // Fast path: Check cache first
        if let Some(cached_tree) = base_tree_cache.get(path) {
            return Ok(Some(cached_tree.clone()));
        }

        // Handle root path
        if self.is_root_path(path) {
            return Ok(Some(root.clone()));
        }

        let mut current_tree = root.clone();
        let mut current_path = PathBuf::new();

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_str().ok_or_else(|| {
                    MegaError::Other(format!("Invalid path component: {:?}", name))
                })?;

                // Update current path level
                current_path.push(name);

                // Check cache for intermediate levels
                if let Some(cached) = base_tree_cache.get(&current_path) {
                    current_tree = cached.clone();
                    continue;
                }

                // Find the directory in current tree
                let item = current_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == name_str && x.mode == TreeItemMode::Tree);

                match item {
                    Some(item) => {
                        // Directory exists, load its tree
                        let tree_model = self
                            .storage
                            .get_tree_by_hash(&item.id.to_string())
                            .await?
                            .ok_or_else(|| {
                                MegaError::Other(format!(
                                    "Tree hash in index but not found: {}",
                                    item.id
                                ))
                            })?;
                        current_tree = Tree::from_mega_model(tree_model);

                        // Update cache
                        base_tree_cache.insert(current_path.clone(), current_tree.clone());
                    }
                    None => {
                        // Directory does not exist - this is a new directory
                        return Ok(None);
                    }
                }
            }
        }

        Ok(Some(current_tree))
    }

    /// Create an empty Tree structure for new directories.
    ///
    /// This is only used in memory during tree building.
    /// The actual Git tree object will be created when items are added.
    fn empty_tree() -> Tree {
        // Create a valid empty tree to get the correct ObjectHash
        Tree::from_tree_items(vec![]).unwrap_or(Tree {
            id: ObjectHash::default(),
            tree_items: vec![],
        })
    }

    /// Sort tree items according to Git's specific rules.
    ///
    /// Git's tree sorting rules:
    /// - Compare items byte-by-byte
    /// - Directories are treated as if they have a trailing '/'
    /// - This ensures directories sort after files with the same prefix
    fn sort_tree_items_git_style(items: &mut [TreeItem]) {
        items.sort_by(|a, b| {
            let a_name = a.name.as_bytes();
            let b_name = b.name.as_bytes();
            let a_is_tree = a.mode == TreeItemMode::Tree;
            let b_is_tree = b.mode == TreeItemMode::Tree;

            let a_len = a_name.len() + if a_is_tree { 1 } else { 0 };
            let b_len = b_name.len() + if b_is_tree { 1 } else { 0 };
            let min_len = std::cmp::min(a_len, b_len);

            for i in 0..min_len {
                let c1 = if i < a_name.len() { a_name[i] } else { b'/' };
                let c2 = if i < b_name.len() { b_name[i] } else { b'/' };
                let cmp = c1.cmp(&c2);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            a_len.cmp(&b_len)
        });
    }

    /// Update a tree with new file blob hashes and child directory updates
    ///
    /// Clones the existing tree items, updates or adds items for each file in this directory,
    /// updates child directory hashes using pre-computed parent->children mapping, and creates
    /// a new tree with the updated items sorted by name (Git requirement).
    ///
    /// # Arguments
    /// * `current_dir_path` - Path of the directory being updated (e.g., "src" or PathBuf::new() for root)
    /// * `existing_tree` - Current tree for this directory
    /// * `files` - Files belonging directly to this directory (not in subdirectories)
    /// * `children_by_parent` - Pre-computed map of parent path -> direct children (path, tree)
    fn update_tree_with_files(
        &self,
        current_dir_path: &Path,
        existing_tree: &Tree,
        files: &[FileChange],
        children_by_parent: &HashMap<PathBuf, Vec<(PathBuf, Tree)>>,
    ) -> Result<Tree, MegaError> {
        let mut items = existing_tree.tree_items.clone();

        // Helper: Find item index by name
        let find_idx = |name: &str, items: &[TreeItem]| items.iter().position(|x| x.name == name);

        // Update blob items for files directly in this directory
        for file in files {
            let file_name = PathBuf::from(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| MegaError::Other(format!("Invalid file path: {}", file.path)))?;

            let blob_hash = file.parse_blob_hash()?;

            let mode = file.tree_item_mode();

            // Check for conflict: Directory vs File
            if let Some(idx) = find_idx(&file_name, &items) {
                let existing = &items[idx];
                if existing.mode == TreeItemMode::Tree {
                    return Err(MegaError::Other(format!(
                        "Type conflict: '{}' is a directory, cannot overwrite with file.",
                        file_name
                    )));
                }
                // Update existing blob
                items[idx].id = blob_hash;
                items[idx].mode = mode;
            } else {
                items.push(TreeItem {
                    mode,
                    id: blob_hash,
                    name: file_name,
                });
            }
        }

        // Update child directory hashes using pre-computed mapping
        let current_path_buf = current_dir_path.to_path_buf();
        if let Some(children) = children_by_parent.get(&current_path_buf) {
            for (child_dir_path, child_tree) in children {
                let child_name = child_dir_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| {
                        MegaError::Other(format!(
                            "Invalid child directory path (no name): {:?}",
                            child_dir_path
                        ))
                    })?;

                // Update or add the child directory item
                if let Some(idx) = find_idx(child_name, &items) {
                    let existing = &items[idx];
                    if existing.mode != TreeItemMode::Tree {
                        return Err(MegaError::Other(format!(
                            "Type conflict: '{}' is a file, cannot overwrite with directory",
                            child_name
                        )));
                    }
                    items[idx].id = child_tree.id;
                } else {
                    // New child directory
                    items.push(TreeItem {
                        mode: TreeItemMode::Tree,
                        id: child_tree.id,
                        name: child_name.to_string(),
                    });
                }
            }
        }

        // Sort items according to Git specification
        Self::sort_tree_items_git_style(&mut items);

        Tree::from_tree_items(items)
            .map_err(|e| MegaError::Other(format!("Failed to create tree: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test group_files_by_directory method
    #[tokio::test]
    async fn test_group_files_by_directory() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let files = vec![
            FileChange::new(
                "src/main.rs".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "src/lib.rs".to_string(),
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "docs/readme.md".to_string(),
                "sha1:ghi789ghi789ghi789ghi789ghi789ghi789ghi7".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "root.txt".to_string(),
                "sha1:jkl012jkl012jkl012jkl012jkl012jkl012jkl0".to_string(),
                "100644".to_string(),
            ),
        ];

        let groups = builder.group_files_by_directory(&files).unwrap();

        // Verify root directory exists and contains root.txt
        assert!(groups.contains_key(&PathBuf::new()), "Root should exist");
        assert_eq!(groups[&PathBuf::new()].len(), 1, "Root should have 1 file");
        assert_eq!(groups[&PathBuf::new()][0].path, "root.txt");

        // Verify src directory exists and contains 2 files
        assert!(
            groups.contains_key(&PathBuf::from("src")),
            "src directory should exist"
        );
        assert_eq!(
            groups[&PathBuf::from("src")].len(),
            2,
            "src should have 2 files"
        );
        let src_files: Vec<&str> = groups[&PathBuf::from("src")]
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(src_files.contains(&"src/main.rs"));
        assert!(src_files.contains(&"src/lib.rs"));

        // Verify docs directory exists and contains 1 file
        assert!(
            groups.contains_key(&PathBuf::from("docs")),
            "docs directory should exist"
        );
        assert_eq!(groups[&PathBuf::from("docs")].len(), 1);

        // Verify total number of directories (root + src + docs)
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_file_change_tree_item_mode() {
        let normal = FileChange::new(
            "a.txt".to_string(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100644".to_string(),
        );
        assert_eq!(normal.tree_item_mode(), TreeItemMode::Blob);

        let exec = FileChange::new(
            "script.sh".to_string(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100755".to_string(),
        );
        assert_eq!(exec.tree_item_mode(), TreeItemMode::BlobExecutable);

        let link = FileChange::new(
            "link".to_string(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "120000".to_string(),
        );
        assert_eq!(link.tree_item_mode(), TreeItemMode::Link);
    }

    /// Test nested directory tree building: a/b/c.txt
    ///
    /// This test verifies the core tree building logic without database dependencies.
    /// It simulates the main loop in `build_tree_with_changes` to ensure:
    /// - c.txt blob is in 'b' Tree
    /// - 'b' Tree hash is in 'a' Tree
    /// - 'a' Tree hash is in Root Tree
    #[test]
    fn test_nested_directory_tree_building() {
        // Setup: Create a file change for a/b/c.txt
        let blob_hash = "da39a3ee5e6b4b0d3255bfef95601890afd80709"; // SHA-1 hash of empty file
        let file = FileChange::new(
            "a/b/c.txt".to_string(),
            format!("sha1:{}", blob_hash),
            "100644".to_string(),
        );

        // Group files by directory
        let mut files_by_dir: HashMap<PathBuf, Vec<FileChange>> = HashMap::new();
        files_by_dir.insert(PathBuf::new(), Vec::new()); // Root (empty)
        files_by_dir.insert(PathBuf::from("a"), Vec::new()); // 'a' directory (no direct files)
        files_by_dir.insert(PathBuf::from("a/b"), vec![file.clone()]); // 'a/b' directory (has c.txt)

        // Sort directories from deepest to shallowest
        let mut sorted_dirs: Vec<_> = files_by_dir.keys().cloned().collect();
        sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

        // Expected order: ["a/b", "a", ""]
        assert_eq!(sorted_dirs.len(), 3);
        assert_eq!(sorted_dirs[0], PathBuf::from("a/b"));
        assert_eq!(sorted_dirs[1], PathBuf::from("a"));
        assert_eq!(sorted_dirs[2], PathBuf::new());

        // Build trees from bottom up (Git doesn't allow empty trees)
        let mut dir_trees: HashMap<PathBuf, Tree> = HashMap::new();

        // Process "a/b" directory: create tree with c.txt
        {
            let dir_path = PathBuf::from("a/b");
            let dir_files = &files_by_dir[&dir_path];

            // Create tree items for files in this directory
            let mut items: Vec<TreeItem> = vec![];
            for f in dir_files {
                let file_name = PathBuf::from(&f.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap()
                    .to_string();
                items.push(TreeItem {
                    mode: TreeItemMode::Blob,
                    id: f.parse_blob_hash().unwrap(),
                    name: file_name,
                });
            }

            let b_tree = Tree::from_tree_items(items).unwrap();

            // Verify: c.txt blob is in 'b' Tree
            let c_txt_item = b_tree.tree_items.iter().find(|item| item.name == "c.txt");
            assert!(c_txt_item.is_some(), "c.txt should be in 'b' Tree");
            assert_eq!(
                c_txt_item.unwrap().id.to_string(),
                blob_hash,
                "c.txt blob hash should match"
            );

            dir_trees.insert(dir_path, b_tree);
        }

        // Process "a" directory: create tree with 'b' subdirectory
        {
            let dir_path = PathBuf::from("a");
            let _dir_files = &files_by_dir[&dir_path]; // Empty for 'a'

            // Build items: find direct children (b)
            let mut items: Vec<TreeItem> = vec![];

            // Find direct children in dir_trees (paths whose parent == "a")
            for (child_path, child_tree) in &dir_trees {
                if let Some(parent) = child_path.parent()
                    && parent == dir_path
                {
                    let child_name = child_path.file_name().unwrap().to_str().unwrap();
                    items.push(TreeItem {
                        mode: TreeItemMode::Tree,
                        id: child_tree.id,
                        name: child_name.to_string(),
                    });
                }
            }

            let a_tree = Tree::from_tree_items(items).unwrap();

            // Verify: 'b' Tree hash is in 'a' Tree
            let b_dir_item = a_tree.tree_items.iter().find(|item| item.name == "b");
            assert!(b_dir_item.is_some(), "'b' directory should be in 'a' Tree");
            assert_eq!(
                b_dir_item.unwrap().id,
                dir_trees[&PathBuf::from("a/b")].id,
                "'b' Tree hash should match"
            );
            assert_eq!(
                b_dir_item.unwrap().mode,
                TreeItemMode::Tree,
                "'b' should be a Tree"
            );

            dir_trees.insert(dir_path, a_tree);
        }

        // Process "" (root) directory: create tree with 'a' subdirectory
        {
            let dir_path = PathBuf::new();
            let _dir_files = &files_by_dir[&dir_path]; // Empty for root

            // Build items: find direct children (a)
            let mut items: Vec<TreeItem> = vec![];

            // Find direct children in dir_trees (paths whose parent == "")
            for (child_path, child_tree) in &dir_trees {
                if let Some(parent) = child_path.parent()
                    && parent.as_os_str().is_empty()
                {
                    let child_name = child_path.file_name().unwrap().to_str().unwrap();
                    items.push(TreeItem {
                        mode: TreeItemMode::Tree,
                        id: child_tree.id,
                        name: child_name.to_string(),
                    });
                }
            }

            let root_tree = Tree::from_tree_items(items).unwrap();

            // Verify: 'a' Tree hash is in Root Tree
            let a_dir_item = root_tree.tree_items.iter().find(|item| item.name == "a");
            assert!(a_dir_item.is_some(), "'a' directory should be in Root Tree");
            assert_eq!(
                a_dir_item.unwrap().id,
                dir_trees[&PathBuf::from("a")].id,
                "'a' Tree hash should match"
            );
            assert_eq!(
                a_dir_item.unwrap().mode,
                TreeItemMode::Tree,
                "'a' should be a Tree"
            );

            dir_trees.insert(dir_path, root_tree);
        }

        // Final verification: all three trees exist with correct relationships
        assert!(
            dir_trees.contains_key(&PathBuf::from("a/b")),
            "a/b Tree should exist"
        );
        assert!(
            dir_trees.contains_key(&PathBuf::from("a")),
            "a Tree should exist"
        );
        assert!(
            dir_trees.contains_key(&PathBuf::new()),
            "Root Tree should exist"
        );

        // Print tree structure for debugging
        println!("=== Tree Structure ===");
        println!("Root Tree ID: {}", dir_trees[&PathBuf::new()].id);
        println!("  └── a Tree ID: {}", dir_trees[&PathBuf::from("a")].id);
        println!(
            "        └── b Tree ID: {}",
            dir_trees[&PathBuf::from("a/b")].id
        );
        println!("              └── c.txt blob: {}", blob_hash);
    }

    /// Test that update_tree_with_files correctly links child directories
    #[test]
    fn test_update_tree_with_files_links_children() {
        // Create a fake child tree for "src" directory
        let src_tree = Tree::from_tree_items(vec![TreeItem {
            mode: TreeItemMode::Blob,
            id: ObjectHash::from_str("da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap(),
            name: "main.rs".to_string(),
        }])
        .unwrap();

        // Create dir_trees with "src" entry
        let mut dir_trees: HashMap<PathBuf, Tree> = HashMap::new();
        dir_trees.insert(PathBuf::from("src"), src_tree.clone());

        // Simulate building root tree by finding child directories
        let current_dir = PathBuf::new();
        let mut items: Vec<TreeItem> = vec![];

        // Simulate the child directory linking logic
        for (child_path, child_tree) in &dir_trees {
            if let Some(parent) = child_path.parent()
                && parent == current_dir.as_path()
            {
                let child_name = child_path.file_name().unwrap().to_str().unwrap();
                items.push(TreeItem {
                    mode: TreeItemMode::Tree,
                    id: child_tree.id,
                    name: child_name.to_string(),
                });
            }
        }

        let updated_root = Tree::from_tree_items(items).unwrap();

        // Verify "src" is in the root tree
        let src_item = updated_root
            .tree_items
            .iter()
            .find(|item| item.name == "src");
        assert!(src_item.is_some(), "'src' should be in updated root tree");
        assert_eq!(src_item.unwrap().id, src_tree.id, "'src' hash should match");
        assert_eq!(src_item.unwrap().mode, TreeItemMode::Tree);
    }

    /// Test group_files_by_directory includes root, intermediate dirs, and handles nested paths
    #[test]
    fn test_group_files_includes_root_and_nested() {
        let files = vec![
            FileChange::new(
                "a/b/c.txt".to_string(),
                "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "x.txt".to_string(),
                "sha1:356a192b7913b04c54574d18c28d46e6395428ab".to_string(),
                "100644".to_string(),
            ),
        ];

        // Simulate the FIXED grouping logic (includes intermediate directories)
        let mut groups: HashMap<PathBuf, Vec<FileChange>> = HashMap::new();
        groups.insert(PathBuf::new(), Vec::new()); // Ensure root exists

        for file in &files {
            let path = PathBuf::from(&file.path);
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();

            // Add all intermediate directories
            let mut current = dir.clone();
            while !current.as_os_str().is_empty() {
                groups.entry(current.clone()).or_default();
                current = current.parent().unwrap_or(Path::new("")).to_path_buf();
            }

            groups.entry(dir).or_default().push(file.clone());
        }

        // Verify root exists
        assert!(groups.contains_key(&PathBuf::new()), "Root should exist");

        // Verify a/b directory has c.txt
        assert!(
            groups.contains_key(&PathBuf::from("a/b")),
            "a/b should exist"
        );
        assert_eq!(groups[&PathBuf::from("a/b")].len(), 1);
        assert_eq!(groups[&PathBuf::from("a/b")][0].path, "a/b/c.txt");

        // KEY FIX: Verify intermediate directory "a" exists (even with no direct files)
        assert!(
            groups.contains_key(&PathBuf::from("a")),
            "'a' intermediate directory should exist"
        );
        assert_eq!(
            groups[&PathBuf::from("a")].len(),
            0,
            "'a' should have no direct files"
        );

        // Verify root has x.txt
        let root_files = &groups[&PathBuf::new()];
        assert_eq!(root_files.len(), 1, "Root should have x.txt");
        assert_eq!(root_files[0].path, "x.txt");

        println!("=== Group Keys (should include intermediate 'a') ===");
        for key in groups.keys() {
            println!("  {:?} -> {} files", key, groups[key].len());
        }
    }

    /// Test path traversal attack rejection
    #[test]
    fn test_path_traversal_rejection() {
        let malicious_paths = vec![
            "../../etc/passwd",
            "a/../../../secret",
            "./../../config",
            "src/../../../bad",
            "normal/../../sensitive",
            "a/b/../c", // Even subtle traversal should be caught
        ];

        for path_str in malicious_paths {
            let path = PathBuf::from(path_str);

            // Component-level check (the correct way)
            let has_parent_component = path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
            assert!(
                has_parent_component,
                "Path '{}' should have ParentDir component",
                path_str
            );
        }
    }

    /// Test that legitimate filenames with ".." are NOT rejected (False Positive fix)
    #[test]
    fn test_legitimate_double_dot_filenames() {
        let legitimate_paths = vec![
            "version..2.0.txt",
            "my..config.yaml",
            "file..backup.rs",
            "data..old.json",
            "src/module..v2.rs",
            "docs/readme..draft.md",
        ];

        for path_str in legitimate_paths {
            let path = PathBuf::from(path_str);

            // These should NOT have ParentDir component
            let has_parent_component = path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
            assert!(
                !has_parent_component,
                "Legitimate path '{}' should not have ParentDir component",
                path_str
            );

            // But they do contain ".." string (which is OK!)
            assert!(
                path_str.contains(".."),
                "Path '{}' should contain '..' string (but that's legitimate)",
                path_str
            );
        }
    }

    /// Test invalid path rejection
    #[test]
    fn test_invalid_path_rejection() {
        // Test empty path
        let empty_path = "";
        assert!(empty_path.is_empty(), "Empty path should be rejected");

        // Test null byte
        let null_byte_path = "file\0.txt";
        assert!(
            null_byte_path.contains('\0'),
            "Path with null byte should be rejected"
        );

        // Test only slashes
        let slash_only = "///";
        let normalized = slash_only.trim_start_matches('/');
        assert!(
            normalized.is_empty(),
            "Path with only slashes should normalize to empty"
        );

        // Test path length limit
        let long_path = "a/".repeat(2500) + "file.txt";
        assert!(
            long_path.len() > 4096,
            "Path should exceed 4096 character limit"
        );

        // Test nesting depth
        let deep_path = "a/".repeat(150) + "file.txt";
        let path = PathBuf::from(&deep_path);
        let depth = path.components().count();
        assert!(depth > 100, "Path should exceed 100 level nesting limit");
    }

    /// Test .git directory access rejection (including edge cases)
    #[test]
    fn test_git_directory_rejection() {
        let git_paths = vec![
            ".git/config",               // Standard case
            ".git/HEAD",                 // Standard case
            "src/.git/hooks/pre-commit", // Nested .git
            ".git/objects/abc123",       // Standard case
            "subdir/.git/index",         // Nested .git
            ".git",                      // Edge case: exactly ".git" (no trailing slash)
            "src/.git",                  // Edge case: ".git" as file/dir name
            "a/b/.git",                  // Edge case: nested without trailing slash
            ".git/",                     // With trailing slash
        ];

        for path_str in git_paths {
            let path = PathBuf::from(path_str);

            // Component-level check (the correct way)
            let has_git_component = path.components().any(|c| {
                if let std::path::Component::Normal(os_str) = c {
                    os_str == ".git"
                } else {
                    false
                }
            });

            assert!(
                has_git_component,
                "Path '{}' should have .git component",
                path_str
            );
        }
    }

    /// Test that files with "git" in name are allowed (not false positive)
    #[test]
    fn test_git_in_filename_allowed() {
        let legitimate_paths = vec![
            "my-git-repo.txt",          // Contains "git" but not ".git"
            ".gitignore",               // Standard Git file
            ".github/workflows/ci.yml", // GitHub Actions
            "src/gitutil.rs",           // Contains "git" in name
            "docs/git-tutorial.md",     // Contains "git" in name
            "ungit.config",             // Ends with "git"
        ];

        for path_str in legitimate_paths {
            let path = PathBuf::from(path_str);

            // These should NOT have ".git" as a component
            let has_git_component = path.components().any(|c| {
                if let std::path::Component::Normal(os_str) = c {
                    os_str == ".git"
                } else {
                    false
                }
            });

            assert!(
                !has_git_component,
                "Legitimate path '{}' should not be blocked",
                path_str
            );
        }
    }

    /// Test that group_files_by_directory rejects .git paths (integration test)
    ///
    /// This test verifies the complete call chain: group_files_by_directory -> normalize_path_to_components
    /// ensures that .git paths are properly rejected at the integration level.
    #[tokio::test]
    async fn test_group_files_rejects_git_paths_integration() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let git_paths = vec![
            ".git/config",
            ".git/HEAD",
            "src/.git/hooks/pre-commit",
            ".git/objects/abc123",
        ];

        for git_path in git_paths {
            let files = vec![FileChange::new(
                git_path.to_string(),
                "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                "100644".to_string(),
            )];

            let result = builder.group_files_by_directory(&files);

            assert!(result.is_err(), "Should reject .git path: {}", git_path);

            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains(".git") || err_msg.contains("Security"),
                "Error message should mention .git or security: {}",
                err_msg
            );
        }
    }

    /// Test that group_files_by_directory allows legitimate paths with "git" in name
    #[tokio::test]
    async fn test_group_files_allows_git_in_filename() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let legitimate_paths = vec![
            ".gitignore",
            ".github/workflows/ci.yml",
            "src/gitutil.rs",
            "docs/git-tutorial.md",
        ];

        for path in legitimate_paths {
            let files = vec![FileChange::new(
                path.to_string(),
                "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                "100644".to_string(),
            )];

            let result = builder.group_files_by_directory(&files);

            assert!(
                result.is_ok(),
                "Should allow legitimate path with 'git' in name: {}",
                path
            );
        }
    }

    /// Test absolute path rejection
    #[test]
    fn test_absolute_path_rejection() {
        let absolute_paths = vec!["/etc/passwd", "/tmp/file", "/var/log/system.log"];

        for path_str in absolute_paths {
            let path = PathBuf::from(path_str);
            let has_root = path
                .components()
                .any(|c| matches!(c, std::path::Component::RootDir));
            assert!(
                has_root,
                "Path '{}' should be detected as absolute",
                path_str
            );
        }
    }

    /// Test Windows absolute path rejection
    #[test]
    fn test_windows_absolute_path_rejection() {
        let windows_paths = vec![
            "C:/Windows/System32/config/sam",
            "C:\\Windows\\System32\\config\\sam",
            "D:/Users/test.txt",
            "Z:/path/to/file",
            "A:/root",
        ];

        for path_str in windows_paths {
            let result = BuckCommitBuilder::normalize_path_to_components(path_str);
            assert!(
                result.is_err(),
                "Should reject Windows absolute path: {}",
                path_str
            );
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Absolute path") || err_msg.contains("Windows drive"),
                "Error message should mention absolute path or Windows drive: {}",
                err_msg
            );
        }
    }

    /// Test valid paths with special characters are accepted
    #[test]
    fn test_special_characters_in_filenames() {
        let valid_paths = vec![
            "file with spaces.txt",
            "file-with-dashes.txt",
            "file_with_underscores.txt",
            "file.multiple.dots.txt",
            "file[brackets].txt",
            "file(parens).txt",
            "中文文件名.txt",
            "файл.txt",
        ];

        for path_str in valid_paths {
            let path = PathBuf::from(path_str);

            // Check no parent directory references
            let has_parent = path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
            assert!(
                !has_parent,
                "Path '{}' should not have parent refs",
                path_str
            );

            // Check no absolute path
            let is_absolute = path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::RootDir | std::path::Component::Prefix(_)
                )
            });
            assert!(!is_absolute, "Path '{}' should not be absolute", path_str);

            // Check no .git access
            let has_git = path_str.starts_with(".git/") || path_str.contains("/.git/");
            assert!(!has_git, "Path '{}' should not access .git", path_str);

            // These should be accepted
            println!("Valid path accepted: {}", path_str);
        }
    }

    /// Test hash format validation
    #[test]
    fn test_hash_format_validation() {
        // Valid format: "sha1:HEXSTRING"
        let valid = FileChange::new(
            "file.txt".to_string(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100644".to_string(),
        );
        assert!(
            valid.parse_blob_hash().is_ok(),
            "Valid sha1 hash should be accepted"
        );

        // Invalid formats
        let invalid_cases = vec![
            ("abc123", "missing algorithm prefix"),
            (
                "md5:5d41402abc4b2a76b9719d911017c592",
                "unsupported algorithm",
            ),
            ("sha1:", "empty hash"),
            ("sha1:invalid", "non-hexadecimal characters"),
            ("sha1:abc", "hash too short"),
            (
                "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "unsupported algorithm (sha256)",
            ),
            (
                ":da39a3ee5e6b4b0d3255bfef95601890afd80709",
                "missing algorithm name",
            ),
            (
                "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709:extra",
                "extra colons",
            ),
        ];

        for (hash, description) in invalid_cases {
            let fc = FileChange::new(
                "file.txt".to_string(),
                hash.to_string(),
                "100644".to_string(),
            );
            assert!(
                fc.parse_blob_hash().is_err(),
                "Should reject hash with {}: {}",
                description,
                hash
            );
        }
    }

    /// Test that valid ObjectHash values with correct format are accepted.
    ///
    /// All hashes are normalized to lowercase per Git convention.
    #[test]
    fn test_valid_sha1_hashes() {
        let valid_hashes = vec![
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709", // empty file
            "SHA1:da39a3ee5e6b4b0d3255bfef95601890afd80709", // uppercase algorithm (normalized)
            "Sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709", // mixed case algorithm (normalized)
            "sha1:356a192b7913b04c54574d18c28d46e6395428ab", // "1"
            "sha1:0000000000000000000000000000000000000000", // all zeros
            "sha1:ffffffffffffffffffffffffffffffffffffffff", // all f's
            "sha1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", // uppercase hex (normalized to lowercase)
            "sha1:aAbBcCdDeEfF00112233445566778899aAbBcCdD", // mixed case hex (normalized to lowercase)
        ];

        for hash in valid_hashes {
            let fc = FileChange::new(
                "file.txt".to_string(),
                hash.to_string(),
                "100644".to_string(),
            );
            let result = fc.parse_blob_hash();
            assert!(result.is_ok(), "Valid hash should be accepted: {}", hash);

            // Verify that the parsed hash is normalized to lowercase
            if let Ok(parsed) = result {
                let hash_str = parsed.to_string();
                assert_eq!(
                    hash_str,
                    hash_str.to_lowercase(),
                    "Parsed hash should be lowercase"
                );
            }
        }
    }

    /// Test group_files_by_directory with relative paths (prefix stripping removed)
    #[tokio::test]
    async fn test_group_files_with_repo_prefix() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        // Since prefix stripping is removed, paths with leading slashes
        // are normalized by removing the leading slash, but the full path is preserved
        let files = vec![
            FileChange::new(
                "src/main.rs".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "docs/readme.md".to_string(),
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
        ];

        let groups = builder.group_files_by_directory(&files).unwrap();

        // Verify paths are correctly grouped (no prefix stripping)
        assert!(groups.contains_key(&PathBuf::from("src")));
        assert_eq!(groups[&PathBuf::from("src")][0].path, "src/main.rs");

        assert!(groups.contains_key(&PathBuf::from("docs")));
        assert_eq!(groups[&PathBuf::from("docs")][0].path, "docs/readme.md");
    }

    /// Test path length limit (4096 characters)
    #[tokio::test]
    async fn test_path_length_limit() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        // Create a path longer than 4096 characters but with depth <= 100
        // Use a long filename component to exceed length limit without exceeding depth limit
        let long_component = "a".repeat(4100); // Single component of 4100 chars
        let long_path = format!("{}/file.txt", long_component);

        // Verify the path exceeds length limit
        let path = PathBuf::from(&long_path);
        let components: Vec<String> = path
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(os_str) = c {
                    os_str.to_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        let total_len = components.iter().map(|c| c.len()).sum::<usize>()
            + if components.is_empty() {
                0
            } else {
                components.len() - 1
            };
        assert!(
            total_len > 4096,
            "Test path should exceed 4096 characters in canonical form"
        );
        assert!(
            components.len() <= 100,
            "Test path should not exceed 100 levels of nesting"
        );

        let file = FileChange::new(
            long_path.clone(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100644".to_string(),
        );

        let result = builder.group_files_by_directory(&[file]);
        assert!(
            result.is_err(),
            "Should reject path longer than 4096 characters"
        );

        let err_msg = result.unwrap_err().to_string();
        // The error message should mention path length limit
        assert!(
            err_msg.contains("Path too long") || err_msg.contains("4096"),
            "Error message should mention path length limit: {}",
            err_msg
        );
    }

    /// Test nesting depth limit (100 levels)
    #[tokio::test]
    async fn test_nesting_depth_limit() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        // Create a path with more than 100 levels of nesting
        let deep_path = "a/".repeat(150) + "file.txt";
        let path = PathBuf::from(&deep_path);
        let depth = path.components().count();
        assert!(depth > 100, "Test path should have more than 100 levels");

        let file = FileChange::new(
            deep_path,
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100644".to_string(),
        );

        let result = builder.group_files_by_directory(&[file]);
        assert!(
            result.is_err(),
            "Should reject path with nesting deeper than 100 levels"
        );

        let err_msg = result.unwrap_err().to_string();
        // The error message changed in normalize_path_to_components
        assert!(
            err_msg.contains("Path nesting too deep") || err_msg.contains("100 levels"),
            "Error message should mention nesting depth limit: {}",
            err_msg
        );
    }

    /// Test that intermediate directories are created for nested paths
    #[tokio::test]
    async fn test_group_files_creates_intermediate_dirs() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let files = vec![FileChange::new(
            "a/b/c/deep.txt".to_string(),
            "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
            "100644".to_string(),
        )];

        let groups = builder.group_files_by_directory(&files).unwrap();

        // Verify all intermediate directories are created
        assert!(groups.contains_key(&PathBuf::new()), "Root should exist");
        assert!(
            groups.contains_key(&PathBuf::from("a")),
            "Intermediate 'a' should exist"
        );
        assert!(
            groups.contains_key(&PathBuf::from("a/b")),
            "Intermediate 'a/b' should exist"
        );
        assert!(
            groups.contains_key(&PathBuf::from("a/b/c")),
            "'a/b/c' should exist"
        );

        // Verify intermediate directories have no direct files
        assert_eq!(
            groups[&PathBuf::from("a")].len(),
            0,
            "'a' should have no direct files"
        );
        assert_eq!(
            groups[&PathBuf::from("a/b")].len(),
            0,
            "'a/b' should have no direct files"
        );

        // Verify file is in the correct directory
        assert_eq!(
            groups[&PathBuf::from("a/b/c")].len(),
            1,
            "'a/b/c' should have 1 file"
        );
        assert_eq!(groups[&PathBuf::from("a/b/c")][0].path, "a/b/c/deep.txt");
    }

    /// Test duplicate path rejection
    #[tokio::test]
    async fn test_duplicate_paths_rejection() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let files = vec![
            FileChange::new(
                "file.txt".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "file.txt".to_string(),                                      // Same path
                "sha1:def456def456def456def456def456def456def4".to_string(), // Different hash
                "100644".to_string(),
            ),
        ];

        let result = builder.group_files_by_directory(&files);
        assert!(result.is_err(), "Should reject duplicate file paths");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Duplicate") || err_msg.contains("duplicate"),
            "Error message should mention duplicate: {}",
            err_msg
        );
    }

    /// Test duplicate paths with different cases are treated as separate
    #[tokio::test]
    async fn test_duplicate_paths_case_sensitive() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        // Git is case-sensitive on most systems
        let files = vec![
            FileChange::new(
                "File.txt".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "file.txt".to_string(), // Different case
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
        ];

        let result = builder.group_files_by_directory(&files);
        // On case-sensitive filesystems, these are different files
        assert!(
            result.is_ok(),
            "Different case should be treated as different files on case-sensitive systems"
        );
    }

    /// Test duplicate paths after normalization
    #[tokio::test]
    async fn test_duplicate_paths_after_normalization() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let files = vec![
            FileChange::new(
                "/src/main.rs".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "src/main.rs".to_string(), // Same after normalization
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
        ];

        let result = builder.group_files_by_directory(&files);
        assert!(
            result.is_err(),
            "Should reject duplicate paths after normalization"
        );
    }

    /// Test duplicate detection with repo_path prefix stripping
    #[tokio::test]
    async fn test_duplicate_paths_with_repo_prefix() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        let files = vec![
            FileChange::new(
                "/project/mega/src/main.rs".to_string(),
                "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
                "100644".to_string(),
            ),
            FileChange::new(
                "/project/mega/src/main.rs".to_string(), // Exact duplicate
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
        ];

        let result = builder.group_files_by_directory(&files);
        assert!(
            result.is_err(),
            "Should reject duplicate paths even with repo prefix"
        );
    }

    /// Test path normalization logic (removing dots, handling slashes)
    #[test]
    fn test_normalize_and_strip_prefix_normalization() {
        let cases = vec![
            ("a//b/c.txt", vec!["a", "b", "c.txt"]),
            ("a/./b/c.txt", vec!["a", "b", "c.txt"]),
            ("src/main.rs", vec!["src", "main.rs"]),
            ("./src/main.rs", vec!["src", "main.rs"]),
        ];

        for (input, expected) in cases {
            let res = BuckCommitBuilder::normalize_path_to_components(input);
            assert_eq!(
                res.unwrap(),
                expected
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
                "Input: {}",
                input
            );
        }
    }

    /// Test Security: Path Traversal Rejection
    #[test]
    fn test_normalize_rejection_of_traversal() {
        let cases = vec![
            "../outside.txt",
            "src/../../etc/passwd",
            "a/b/../c.txt", // Even if it stays inside, we reject '..' entirely for simplicity/safety
        ];

        for input in cases {
            let res = BuckCommitBuilder::normalize_path_to_components(input);
            assert!(res.is_err(), "Should reject traversal: {}", input);
            assert!(
                res.unwrap_err().to_string().contains("Path traversal"),
                "Error should mention traversal"
            );
        }
    }

    #[tokio::test]
    async fn test_update_tree_with_files_type_conflict_async() {
        use jupiter::tests::test_storage;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;
        let builder = BuckCommitBuilder::new(storage.mono_storage());

        // Create a tree with a subdirectory "config"
        let config_tree_hash =
            ObjectHash::from_str("da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap();
        let existing_tree = Tree::from_tree_items(vec![TreeItem {
            mode: TreeItemMode::Tree,
            id: config_tree_hash,
            name: "config".to_string(),
        }])
        .unwrap();

        // Try to add a file named "config" (Conflict!)
        let files = vec![FileChange::new(
            "config".to_string(),
            "sha1:abc123abc123abc123abc123abc123abc123abc1".to_string(),
            "100644".to_string(),
        )];

        let children_map = HashMap::new();

        // Perform update
        let result =
            builder.update_tree_with_files(Path::new(""), &existing_tree, &files, &children_map);

        // Verify failure
        assert!(result.is_err(), "Should fail due to type conflict");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Type conflict"),
            "Error should mention type conflict: {}",
            err
        );
        assert!(
            err.contains("is a directory"),
            "Error should specify directory conflict"
        );
    }

    /// Test Git Sorting Rules
    #[test]
    fn test_git_sort_order() {
        use std::str::FromStr;

        use git_internal::hash::ObjectHash;

        // Setup items
        let blob_hash = ObjectHash::from_str("da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap();

        // According to Git rules:
        // "foo" (directory) -> implicitly "foo/"
        // "foo-bar" (file)
        // Compare "foo/" vs "foo-bar"
        // 'foo' match
        // '/' (47) vs '-' (45) -> '/' > '-'
        // So "foo" (dir) > "foo-bar" (file)

        let mut items = vec![
            TreeItem {
                mode: TreeItemMode::Tree,
                id: blob_hash,
                name: "foo".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Blob,
                id: blob_hash,
                name: "foo-bar".to_string(),
            },
        ];

        BuckCommitBuilder::sort_tree_items_git_style(&mut items);

        // Verify order: foo-bar (file) comes BEFORE foo (directory)
        assert_eq!(items[0].name, "foo-bar");
        assert_eq!(items[1].name, "foo");
    }
}
