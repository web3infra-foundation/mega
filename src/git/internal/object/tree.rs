//! In Git, a tree object is used to represent the state of a directory at a specific point in time.
//! It stores information about the files and directories within that directory, including their names,
//! permissions, and the IDs of the objects that represent their contents.
//!
//! A tree object can contain other tree objects as well as blob objects, which represent the contents
//! of individual files. The object IDs of these child objects are stored within the tree object itself.
//!
//! When you make a commit in Git, you create a new tree object that represents the state of the
//! repository at that point in time. The parent of the new commit is typically the tree object
//! representing the previous state of the repository.
//!
//! Git uses the tree object to efficiently store and manage the contents of a repository. By
//! representing the contents of a directory as a tree object, Git can quickly determine which files
//! have been added, modified, or deleted between two points in time. This allows Git to perform
//! operations like merging and rebasing more quickly and accurately.
//!
use std::fmt::Display;

use colored::Colorize;

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;

/// In Git, the mode field in a tree object's entry specifies the type of the object represented by
/// that entry. The mode is a three-digit octal number that encodes both the permissions and the
/// type of the object. The first digit specifies the object type, and the remaining two digits
/// specify the file mode or permissions.
#[allow(unused)]
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum TreeItemMode {
    Blob,
    BlobExecutable,
    Tree,
    Commit,
    Link,
}

impl Display for TreeItemMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _print = match *self {
            TreeItemMode::Blob => "blob",
            TreeItemMode::BlobExecutable => "blob executable",
            TreeItemMode::Tree => "tree",
            TreeItemMode::Commit => "commit",
            TreeItemMode::Link => "link",
        };
        write!(f, "{}", String::from(_print).blue())
    }
}

impl TreeItemMode {
    #[allow(unused)]
    /// 32-bit mode, split into (high to low bits):
    /// - 4-bit object type: valid values in binary are 1000 (regular file), 1010 (symbolic link) and 1110 (gitlink)
    /// - 3-bit unused
    /// - 9-bit unix permission: Only 0755 and 0644 are valid for regular files. Symbolic links and gitlink have value 0 in this field.
    pub fn to_bytes(self) -> &'static [u8] {
        match self {
            TreeItemMode::Blob => b"100644",
            TreeItemMode::BlobExecutable => b"100755",
            TreeItemMode::Link => b"120000",
            TreeItemMode::Tree => b"040000",
            TreeItemMode::Commit => b"160000",
        }
    }

    /// Convert a 32-bit mode to a TreeItemType
    ///
    /// |0100000000000000| (040000)| Directory|
    /// |1000000110100100| (100644)| Regular non-executable file|
    /// |1000000110110100| (100664)| Regular non-executable group-writeable file|
    /// |1000000111101101| (100755)| Regular executable file|
    /// |1010000000000000| (120000)| Symbolic link|
    /// |1110000000000000| (160000)| Gitlink|
    /// ---
    /// # GitLink
    /// Gitlink, also known as a submodule, is a feature in Git that allows you to include a Git
    /// repository as a subdirectory within another Git repository. This is useful when you want to
    /// incorporate code from another project into your own project, without having to manually copy
    /// the code into your repository.
    ///
    /// When you add a submodule to your Git repository, Git stores a reference to the other
    /// repository at a specific commit. This means that your repository will always point to a
    /// specific version of the other repository, even if changes are made to the submodule's code
    /// in the future.
    ///
    /// To work with a submodule in Git, you use commands like git submodule add, git submodule
    /// update, and git submodule init. These commands allow you to add a submodule to your repository,
    /// update it to the latest version, and initialize it for use.
    ///
    /// Submodules can be a powerful tool for managing dependencies between different projects and
    /// components. However, they can also add complexity to your workflow, so it's important to
    /// understand how they work and when to use them.
    #[allow(unused)]
    pub fn tree_item_type_from(mode: &[u8]) -> Result<TreeItemMode, GitError> {
        Ok(match mode {
            b"040000" => TreeItemMode::Tree,
            b"100644" => TreeItemMode::Blob,
            b"100755" => TreeItemMode::BlobExecutable,
            b"120000" => TreeItemMode::Link,
            b"160000" => TreeItemMode::Commit,
            b"100664" => TreeItemMode::Blob,
            b"100640" => TreeItemMode::Blob,
            _ => {
                return Err(GitError::InvalidTreeItem(
                    String::from_utf8(mode.to_vec()).unwrap(),
                ));
            }
        })
    }
}

/// A tree object contains a list of entries, one for each file or directory in the tree. Each entry
/// in the file represents an entry in the tree, and each entry has the following format:
///
/// ```bash
/// <mode> <name>\0<binary object ID>
/// ```
/// - `<mode>` is the mode of the object, represented as a six-digit octal number. The first digit
/// represents the object type (tree, blob, etc.), and the remaining digits represent the file mode or permissions.
/// - `<name>` is the name of the object.
/// - `\0` is a null byte separator.
/// - `<binary object ID>` is the ID of the object that represents the contents of the file or
/// directory, represented as a binary SHA-1 hash.
///
/// # Example
/// ```bash
/// 100644 hello-world\0<blob object ID>
/// 040000 data\0<tree object ID>
/// ```
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct TreeItem {
    pub mode: TreeItemMode,
    pub id: Hash,
    pub name: String,
}

impl Display for TreeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.mode,
            self.name,
            self.id.to_string().blue()
        )
    }
}

impl TreeItem {
    /// Create a new TreeItem from a mode, id and name
    #[allow(unused)]
    pub fn new(mode: TreeItemMode, id: Hash, name: String) -> Self {
        TreeItem {
            mode,
            id,
            name,
        }
    }

    #[allow(unused)]
    pub fn new_from_bytes(bytes: &[u8]) -> Result<Self, GitError> {
        let mut parts = bytes.splitn(2, |b| *b == b' ');
        let mode = parts.next().unwrap();
        let rest = parts.next().unwrap();
        let mut parts = rest.splitn(2, |b| *b == b'\0');
        let name = parts.next().unwrap();
        let id = parts.next().unwrap();

        Ok(TreeItem {
            mode: TreeItemMode::tree_item_type_from(mode)?,
            id: Hash::from_bytes(id),
            name: String::from_utf8(name.to_vec())?,
        })
    }

    /// Convert a TreeItem to a byte vector
    #[allow(unused)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.mode.to_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(b'\0');
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes
    }
}

/// A tree object is a Git object that represents a directory. It contains a list of entries, one
/// for each file or directory in the tree.
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Tree {
    pub meta: Meta,
    pub name: String,
    pub tree_items: Vec<TreeItem>,
}

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Tree: {}", self.meta.id.to_string().blue())?;
        for item in &self.tree_items {
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

impl Tree {
    #[allow(unused)]
    pub fn empty_tree_hash() -> Hash {
        Hash::default()
    }

    /// Generate a TreeItem from Tree object.
    /// This is used to generate a TreeItem for the parent directory of the current Tree object.
    #[allow(unused)]
    pub fn generate_self_2tree_item(&self) -> Result<TreeItem, GitError> {
        Ok(TreeItem::new(
            TreeItemMode::Tree,
            self.meta.id,
            self.name.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tree_item_new() {
        use crate::git::hash::Hash;

        let tree_item = super::TreeItem::new(
            super::TreeItemMode::Blob,
            Hash::new_from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d"),
            "hello-world".to_string(),
        );

        assert_eq!(tree_item.mode, super::TreeItemMode::Blob);
        assert_eq!(tree_item.id.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }

    #[test]
    fn test_tree_item_to_bytes() {
        use crate::git::hash::Hash;

        let tree_item = super::TreeItem::new(
            super::TreeItemMode::Blob,
            Hash::new_from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d"),
            "hello-world".to_string(),
        );

        let bytes = tree_item.to_bytes();
        assert_eq!(bytes, vec![49, 48, 48, 54, 52, 52, 32, 104, 101, 108, 108, 111, 45, 119, 111, 114, 108, 100, 0, 138, 182, 134, 234, 254, 177, 244, 71, 2, 115, 140, 139, 15, 36, 242, 86, 124, 54, 218, 109]);
    }

    #[test]
    fn test_tree_item_from_bytes() {
        use crate::git::hash::Hash;

        let item = super::TreeItem::new(
            super::TreeItemMode::Blob,
            Hash::new_from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d"),
            "hello-world".to_string(),
        );

        let bytes = item.to_bytes();
        let tree_item = super::TreeItem::new_from_bytes(bytes.as_slice()).unwrap();

        assert_eq!(tree_item.mode, super::TreeItemMode::Blob);
        assert_eq!(tree_item.id.to_plain_str(), item.id.to_plain_str());
    }

    #[test]
    fn test_empty_tree_hash() {
        let hash = super::Tree::empty_tree_hash();
        assert_eq!(hash.to_plain_str(), "0000000000000000000000000000000000000000");
    }
}