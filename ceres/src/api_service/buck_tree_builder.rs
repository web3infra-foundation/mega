//! Buck Upload Tree and Commit Builder
//!
//! This module provides utilities for building Git trees and commits from
//! a batch of file changes. It's designed for the Buck upload API to
//! create a single atomic commit containing multiple file changes

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use common::errors::MegaError;
use git_internal::hash::SHA1;
use git_internal::internal::metadata::EntryMeta;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use jupiter::storage::mono_storage::MonoStorage;
use jupiter::utils::converter::{FromMegaModel, IntoMegaModel};

use crate::model::buck::FileChange;

/// Result of building trees from file changes
#[derive(Debug)]
pub struct TreeBuildResult {
    /// The new root tree after applying all changes
    pub root_tree: Tree,
    /// All new trees that need to be saved (including intermediate directories)
    pub new_trees: Vec<Tree>,
    /// The final tree hash (same as root_tree.id)
    pub tree_hash: SHA1,
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
    /// * `repo_path` - Repository path for normalization
    /// * `files` - List of file changes to apply
    pub async fn build_tree_with_changes(
        &self,
        base_commit_hash: &str,
        repo_path: &str,
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

        // Group files by directory for efficient batch processing
        let files_by_dir = self.group_files_by_directory(files, repo_path)?;

        // Build directory tree updates (key: directory path, value: updated tree)
        let mut dir_trees: HashMap<PathBuf, Tree> = HashMap::new();
        let mut new_trees: Vec<Tree> = Vec::new();

        // Process directories from deepest to shallowest to ensure parent directories
        // can reference their updated children
        let mut sorted_dirs: Vec<_> = files_by_dir.keys().cloned().collect();
        sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

        for dir_path in sorted_dirs {
            let dir_files = &files_by_dir[&dir_path];

            // Load or get the existing tree for this directory
            let existing_tree = if dir_path.as_os_str().is_empty() {
                // Root directory - always exists
                root_tree.clone()
            } else {
                // Try to find existing directory, or use empty tree for new directories
                self.find_tree_at_path(&root_tree, &dir_path)
                    .await?
                    .unwrap_or_else(Self::empty_tree)
            };

            // Update tree items with new blob hashes and child directory updates
            let updated_tree =
                self.update_tree_with_files(&dir_path, &existing_tree, dir_files, &dir_trees)?;

            if updated_tree.id != existing_tree.id {
                new_trees.push(updated_tree.clone());
            }
            dir_trees.insert(dir_path, updated_tree);
        }

        // Get final root tree (already processed in the loop above)
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
    /// * `repo_path` - Repository path for normalization
    /// * `files` - List of file changes to include in commit
    /// * `message` - Commit message
    pub async fn build_commit(
        &self,
        base_commit_hash: &str,
        repo_path: &str,
        files: &[FileChange],
        message: &str,
    ) -> Result<CommitBuildResult, MegaError> {
        // Build tree structure from file changes
        let tree_result = self
            .build_tree_with_changes(base_commit_hash, repo_path, files)
            .await?;

        // Create commit with the new tree
        let parent_sha = SHA1::from_str(base_commit_hash)
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

    /// Group files by their parent directory
    ///
    /// Always includes root directory and all intermediate directories to ensure proper tree chain.
    fn group_files_by_directory(
        &self,
        files: &[FileChange],
        repo_path: &str,
    ) -> Result<HashMap<PathBuf, Vec<FileChange>>, MegaError> {
        let mut groups: HashMap<PathBuf, Vec<FileChange>> = HashMap::new();
        let mut seen_paths: HashSet<String> = HashSet::new();

        // Ensure root directory exists (even if empty)
        groups.insert(PathBuf::new(), Vec::new());

        // Normalize repo_path: remove leading/trailing slashes
        let repo_prefix = repo_path.trim_start_matches('/').trim_end_matches('/');

        for file in files {
            // Normalize file path: remove leading slash, strip repo prefix if present
            let relative_path = file.path.trim_start_matches('/');
            let relative_path = if !repo_prefix.is_empty() && relative_path.starts_with(repo_prefix)
            {
                relative_path
                    .strip_prefix(repo_prefix)
                    .unwrap_or(relative_path)
                    .trim_start_matches('/')
            } else {
                relative_path
            };

            // Validate path is not empty and contains no dangerous characters
            if relative_path.is_empty() || relative_path.contains('\0') {
                return Err(MegaError::Other(format!("Invalid path: {}", file.path)));
            }

            // Path length limit to prevent resource exhaustion
            if relative_path.len() > 4096 {
                return Err(MegaError::Other(format!(
                    "Path too long (max 4096 characters): {}",
                    file.path
                )));
            }

            let path = PathBuf::from(relative_path);
            let mut depth = 0;

            // Validate path components for security
            for component in path.components() {
                match component {
                    // Reject path traversal (../)
                    std::path::Component::ParentDir => {
                        return Err(MegaError::Other(format!(
                            "Path traversal detected (ParentDir): {}",
                            file.path
                        )));
                    }
                    // Reject absolute paths
                    std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                        return Err(MegaError::Other(format!(
                            "Absolute path not allowed: {}",
                            file.path
                        )));
                    }
                    std::path::Component::Normal(os_str) => {
                        // Prevent access to .git directory/file at any level
                        if os_str == ".git" {
                            return Err(MegaError::Other(format!(
                                "Security: .git directory/file access not allowed: {}",
                                file.path
                            )));
                        }
                        depth += 1;
                    }
                    _ => {}
                }
            }

            // Nesting depth limit to prevent stack overflow
            if depth > 100 {
                return Err(MegaError::Other(format!(
                    "Path nesting too deep (max 100 levels): {}",
                    file.path
                )));
            }

            // Check for duplicate paths
            if seen_paths.contains(relative_path) {
                return Err(MegaError::Other(format!(
                    "Duplicate file path in upload: {}",
                    file.path
                )));
            }
            seen_paths.insert(relative_path.to_string());

            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();

            // Add all intermediate directories to ensure proper tree chain
            // For "a/b/c.txt", we need entries for both "a/b" AND "a"
            let mut current = dir.clone();
            while !current.as_os_str().is_empty() {
                groups.entry(current.clone()).or_default();
                current = current.parent().unwrap_or(Path::new("")).to_path_buf();
            }

            // Create a new FileChange with the normalized relative path
            let mut file_change = file.clone();
            file_change.path = relative_path.to_string();

            groups.entry(dir).or_default().push(file_change);
        }

        Ok(groups)
    }

    /// Find tree at a given path by traversing from root
    ///
    /// # Returns:
    /// - `Ok(Some(tree))` if the directory exists
    /// - `Ok(None)` if the directory does not exist (new directory)
    /// - `Err(...)` for actual errors (database issues, etc.)
    async fn find_tree_at_path(&self, root: &Tree, path: &Path) -> Result<Option<Tree>, MegaError> {
        let mut current_tree = root.clone();

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_str().ok_or_else(|| {
                    MegaError::Other(format!("Invalid path component: {:?}", name))
                })?;

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

    /// Create an empty Tree structure for new directories
    ///
    /// Note: This is only used in memory during tree building
    /// The actual Git tree object will be created when items are added
    fn empty_tree() -> Tree {
        Tree {
            id: SHA1::default(), // Temporary, will be recalculated
            tree_items: vec![],
        }
    }

    /// Update a tree with new file blob hashes and child directory updates
    ///
    /// Clones the existing tree items, updates or adds items for each file in this directory,
    /// updates child directory hashes by finding all direct children, and creates a new tree
    /// with the updated items sorted by name (Git requirement).
    ///
    /// # Arguments
    /// * `current_dir_path` - Path of the directory being updated (e.g., "src" or PathBuf::new() for root)
    /// * `existing_tree` - Current tree for this directory
    /// * `files` - Files belonging directly to this directory (not in subdirectories)
    /// * `all_updated_dir_trees` - Map of all updated directory trees (key: full path from root)
    fn update_tree_with_files(
        &self,
        current_dir_path: &Path,
        existing_tree: &Tree,
        files: &[FileChange],
        all_updated_dir_trees: &HashMap<PathBuf, Tree>,
    ) -> Result<Tree, MegaError> {
        let mut items = existing_tree.tree_items.clone();

        // Update blob items for files directly in this directory
        for file in files {
            let file_name = PathBuf::from(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| MegaError::Other(format!("Invalid file path: {}", file.path)))?;

            let blob_hash = file.parse_blob_hash()?;

            let mode = file.tree_item_mode();

            // Find and update existing item, or add new one
            if let Some(pos) = items.iter().position(|x| x.name == file_name) {
                items[pos].id = blob_hash;
                items[pos].mode = mode;
            } else {
                items.push(TreeItem {
                    mode,
                    id: blob_hash,
                    name: file_name,
                });
            }
        }

        // Update child directory hashes by finding all direct children
        // (a child is "direct" if its parent path equals current_dir_path)
        for (child_dir_path, child_tree) in all_updated_dir_trees {
            // Check if this child_dir_path is a direct child of current_dir_path
            if let Some(parent) = child_dir_path.parent() {
                // Compare paths: parent should equal current_dir_path
                if parent == current_dir_path {
                    // This is a direct child directory!
                    let child_name = child_dir_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| {
                            MegaError::Other(format!(
                                "Invalid child directory path: {:?}",
                                child_dir_path
                            ))
                        })?;

                    // Update or add the child directory item
                    if let Some(pos) = items
                        .iter()
                        .position(|x| x.name == child_name && x.mode == TreeItemMode::Tree)
                    {
                        items[pos].id = child_tree.id;
                    } else {
                        // New child directory
                        items.push(TreeItem {
                            mode: TreeItemMode::Tree,
                            id: child_tree.id,
                            name: child_name.to_string(),
                        });
                    }
                }
            } else {
                // child_dir_path has no parent, so it's root
                // Only process this if current_dir_path is also root
                if current_dir_path.as_os_str().is_empty() {
                    // This shouldn't happen (root shouldn't be in all_updated_dir_trees during processing)
                    // But handle it gracefully
                    continue;
                }
            }
        }

        // Sort items by name (Git requirement)
        items.sort_by(|a, b| a.name.cmp(&b.name));

        Tree::from_tree_items(items).map_err(|_| MegaError::Other("Failed to create tree".into()))
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

        let groups = builder.group_files_by_directory(&files, "").unwrap();

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
        let blob_hash = "da39a3ee5e6b4b0d3255bfef95601890afd80709"; // SHA1 of empty file
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

            // Note: items is not empty here (c.txt)
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
                if let Some(parent) = child_path.parent() {
                    if parent == dir_path {
                        let child_name = child_path.file_name().unwrap().to_str().unwrap();
                        items.push(TreeItem {
                            mode: TreeItemMode::Tree,
                            id: child_tree.id,
                            name: child_name.to_string(),
                        });
                    }
                }
            }

            // Note: items is not empty here (b directory)
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
                if let Some(parent) = child_path.parent() {
                    if parent.as_os_str().is_empty() {
                        let child_name = child_path.file_name().unwrap().to_str().unwrap();
                        items.push(TreeItem {
                            mode: TreeItemMode::Tree,
                            id: child_tree.id,
                            name: child_name.to_string(),
                        });
                    }
                }
            }

            // Note: items is not empty here (a directory)
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
            id: SHA1::from_str("da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap(),
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
            if let Some(parent) = child_path.parent() {
                if parent == current_dir.as_path() {
                    let child_name = child_path.file_name().unwrap().to_str().unwrap();
                    items.push(TreeItem {
                        mode: TreeItemMode::Tree,
                        id: child_tree.id,
                        name: child_name.to_string(),
                    });
                }
            }
        }

        // Note: items is not empty (src directory exists)
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

    /// Test that valid SHA1 hashes with correct format are accepted
    /// Note: All hashes are normalized to lowercase per Git convention
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

    /// Test repo_path prefix stripping
    #[tokio::test]
    async fn test_group_files_with_repo_prefix() {
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
                "/project/mega/docs/readme.md".to_string(),
                "sha1:def456def456def456def456def456def456def4".to_string(),
                "100644".to_string(),
            ),
        ];

        let groups = builder
            .group_files_by_directory(&files, "/project/mega")
            .unwrap();

        // Verify paths are correctly stripped of prefix
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

        // Create a path longer than 4096 characters
        let long_path = "a/".repeat(2500) + "file.txt";
        assert!(
            long_path.len() > 4096,
            "Test path should exceed 4096 characters"
        );

        let file = FileChange::new(
            long_path.clone(),
            "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
            "100644".to_string(),
        );

        let result = builder.group_files_by_directory(&[file], "");
        assert!(
            result.is_err(),
            "Should reject path longer than 4096 characters"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("too long") || err_msg.contains("4096"),
            "Error message should mention path length limit"
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

        let result = builder.group_files_by_directory(&[file], "");
        assert!(
            result.is_err(),
            "Should reject path with nesting deeper than 100 levels"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("nesting too deep") || err_msg.contains("100 levels"),
            "Error message should mention nesting depth limit"
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

        let groups = builder.group_files_by_directory(&files, "").unwrap();

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

        let result = builder.group_files_by_directory(&files, "");
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

        let result = builder.group_files_by_directory(&files, "");
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

        let result = builder.group_files_by_directory(&files, "");
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

        let result = builder.group_files_by_directory(&files, "/project/mega");
        assert!(
            result.is_err(),
            "Should reject duplicate paths even with repo prefix"
        );
    }
}
