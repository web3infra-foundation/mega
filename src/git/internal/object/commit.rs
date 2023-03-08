//!
//!
//!
//!
//!
use std::fmt::Display;

use bstr::ByteSlice;

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::signature::Signature;

#[allow(unused)]
#[derive(Eq, Debug, Clone)]
pub struct Commit {
    pub meta: Meta,
    pub tree_id: Hash,
    pub parent_tree_ids: Vec<Hash>,
    pub author: Signature,
    pub committer: Signature,
    pub message: String,
}

impl PartialEq for Commit {
    fn eq(&self, other: &Self) -> bool {
        self.meta.id == other.meta.id
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "tree: {}", self.tree_id)?;
        for parent in self.parent_tree_ids.iter() {
            writeln!(f, "parent: {}", parent)?;
        }
        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;
        writeln!(f, "{}", self.message)
    }
}

impl Commit {
    /// Create a new commit object from a meta object
    #[allow(unused)]
    pub fn new_from_meta(meta: Meta) -> Result<Self, GitError> {
        let mut commit = meta.data.clone();

        // Find the tree id and remove it from the data
        let tree_end = commit.find_byte(0x0a).unwrap();
        let tree_id = Hash::new_from_str(
            String::from_utf8(commit[5..tree_end].to_owned()).unwrap().as_str());
        commit = commit[tree_end + 1..].to_vec();

        // Find the parent tree ids and remove them from the data
        let author_begin = commit.find("author").unwrap();
        let parent_tree_ids: Vec<Hash> = commit[..author_begin]
            .find_iter("parent")
            .map(|parent| {
                let parent_end = commit[parent..].find_byte(0x0a).unwrap();
                Hash::new_from_str(
                    String::from_utf8(commit[parent + 7..parent + parent_end].to_owned()).unwrap().as_str())
            })
            .collect();
        commit = commit[author_begin..].to_vec();

        // Find the author and committer and remove them from the data
        let author = Signature::new_from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec())?;
        commit = commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec();
        let committer = Signature::new_from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec())?;

        // The rest is the message
        let message = String::from_utf8(commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec()).unwrap();

        Ok(Commit {
            meta,
            tree_id,
            parent_tree_ids,
            author,
            committer,
            message,
        })
    }

    #[allow(unused)]
    pub fn new_from_file(path: &str) -> Result<Self, GitError> {
        let meta = Meta::new_from_file(path)?;

        Commit::new_from_meta(meta)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_from_file_without_parent() {
        use std::env;
        use std::path::PathBuf;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/c5/170dd0aae2dc2a9142add9bb24597d326714d7");

        let commit = super::Commit::new_from_file(source.to_str().unwrap()).unwrap();

        assert_eq!(commit.meta.id.to_plain_str(), "c5170dd0aae2dc2a9142add9bb24597d326714d7");
    }

    #[test]
    fn test_new_from_file_with_parent() {
        use std::env;
        use std::path::PathBuf;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/4b/00093bee9b3ef5afc5f8e3645dc39cfa2f49aa");

        let commit = super::Commit::new_from_file(source.to_str().unwrap()).unwrap();

        assert_eq!(commit.parent_tree_ids.len(), 1);
        assert_eq!(commit.meta.id.to_plain_str(), "4b00093bee9b3ef5afc5f8e3645dc39cfa2f49aa");
    }
}