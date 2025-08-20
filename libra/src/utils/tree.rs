use std::collections::HashMap;
use std::path::Path;

use mercury::errors::GitError;
use mercury::internal::index::Index;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::ObjectTrait;

use crate::utils::object;

/// Creates a `Tree` object from the entries in an `Index`.
pub fn create_tree_from_index(index: &Index) -> Result<Tree, GitError> {
    let mut tree_items = Vec::new();

    // Convert IndexEntries to TreeItems
    for entry in index.tracked_entries(0) {
        // Stage 0 for normal files
        let mode = match entry.mode & 0o170000 {
            // Check file type from mode
            0o100000 => {
                // Regular file
                if entry.mode & 0o111 != 0 {
                    TreeItemMode::BlobExecutable
                } else {
                    TreeItemMode::Blob
                }
            }
            0o120000 => TreeItemMode::Link,
            _ => {
                // For simplicity, default to Blob. A full implementation would handle more types.
                TreeItemMode::Blob
            }
        };

        tree_items.push(TreeItem::new(mode, entry.hash, entry.name.clone()));
    }

    // Git tree entries must be sorted by name.
    tree_items.sort_by(|a, b| a.name.cmp(&b.name));

    Tree::from_tree_items(tree_items)
}

/// Helper function to recursively get all files from a tree.
pub fn get_tree_files_recursive(
    tree: &Tree,
    git_dir: &Path,
    current_path: &Path,
) -> Result<HashMap<String, TreeItem>, String> {
    let mut files = HashMap::new();
    for item in &tree.tree_items {
        let item_path = current_path.join(&item.name);
        let item_path_str = item_path
            .to_str()
            .ok_or_else(|| format!("Invalid path: {:?}", item_path))?
            .to_string();

        if item.mode == TreeItemMode::Tree {
            let subtree_data =
                object::read_git_object(git_dir, &item.id).map_err(|e| e.to_string())?;
            let subtree = Tree::from_bytes(&subtree_data, item.id).map_err(|e| e.to_string())?;
            let sub_files = get_tree_files_recursive(&subtree, git_dir, &item_path)?;
            files.extend(sub_files);
        } else {
            files.insert(item_path_str, item.clone());
        }
    }
    Ok(files)
}