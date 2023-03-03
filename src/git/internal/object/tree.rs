//!
//!
//!
//!
//!
use std::fmt::Display;

use colored::Colorize;

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;

#[allow(unused)]
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum TreeItemType {
    Blob,
    BlobExecutable,
    Tree,
    Commit,
    Link,
}

impl Display for TreeItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _print = match *self {
            TreeItemType::Blob => "blob",
            TreeItemType::BlobExecutable => "blob executable",
            TreeItemType::Tree => "tree",
            TreeItemType::Commit => "commit",
            TreeItemType::Link => "link",
        };
        write!(f, "{}", String::from(_print).blue())
    }
}

impl TreeItemType {
    #[allow(unused)]
    pub(crate) fn to_bytes(self) -> &'static [u8] {
        match self {
            TreeItemType::Blob => b"100644",
            TreeItemType::BlobExecutable => b"100755",
            TreeItemType::Tree => b"40000",
            TreeItemType::Link => b"120000",
            TreeItemType::Commit => b"160000",
        }
    }

    #[allow(unused)]
    pub(crate) fn tree_item_type_from(mode: &[u8]) -> Result<TreeItemType, GitError> {
        Ok(match mode {
            b"40000" => TreeItemType::Tree,
            b"100644" => TreeItemType::Blob,
            b"100755" => TreeItemType::BlobExecutable,
            b"120000" => TreeItemType::Link,
            b"160000" => TreeItemType::Commit,
            b"100664" => TreeItemType::Blob,
            b"100640" => TreeItemType::Blob,
            _ => {
                return Err(GitError::InvalidTreeItem(
                    String::from_utf8(mode.to_vec()).unwrap(),
                ));
            }
        })
    }
}

#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct TreeItem {
    pub mode: Vec<u8>,
    pub item_type: TreeItemType,
    pub id: Hash,
    pub filename: String,
}

#[allow(unused)]
pub struct Tree {
    pub meta: Meta,
    pub tree_items: Vec<TreeItem>,
}

impl Tree {
    #[allow(unused)]
    pub fn empty_tree_hash() -> Hash {
        Hash::default()
    }
}

mod tests {
    #[test]
    fn test_empty_tree_hash() {
        let hash = super::Tree::empty_tree_hash();
        assert_eq!(hash.to_plain_str(), "0000000000000000000000000000000000000000");
    }
}