//! The Commit object is  is a data structure used to represent a specific version of a project's
//! files at a particular point in time. In Git, the commit object is a fundamental data structure
//! that is used to track changes to a repository's files over time. Whenever a developer makes
//! changes to the files in a repository, they create a new commit object that records those changes.
//!
//! Each commit object in Git contains the following information:
//!
//! - A unique SHA-1 hash that identifies the commit.
//! - The author and committer of the commit (which may be different people).
//! - The date and time the commit was made.
//! - A commit message that describes the changes made in the commit.
//! - A reference to the parent commit or commits (in the case of a merge commit) that the new commit is based on.
//! - The contents of the files in the repository at the time the commit was made.
//!
//!
//!
use std::fmt::Display;

use bstr::ByteSlice;

use entity::commit;

use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::object::meta::Meta;
use crate::internal::object::signature::Signature;
use crate::internal::object::ObjectT;
use crate::internal::ObjectType;

/// The `Commit` struct is used to represent a commit object.
///
/// - The tree object SHA points to the top level tree for this commit, which reflects the complete
/// state of the repository at the time of the commit. The tree object in turn points to blobs and
/// subtrees which represent the files in the repository.
/// - The parent commit SHAs allow Git to construct a linked list of commits and build the full
/// commit history. By chaining together commits in this fashion, Git is able to represent the entire
/// history of a repository with a single commit object at its root.
/// - The author and committer fields contain the name, email address, timestamp and timezone.
/// - The message field contains the commit message, which maybe include signed or DCO.
#[allow(unused)]
#[derive(Eq, Debug, Clone)]
pub struct Commit {
    pub id: Hash,
    pub tree_id: Hash,
    pub parent_tree_ids: Vec<Hash>,
    pub author: Signature,
    pub committer: Signature,
    pub message: String,
}

impl PartialEq for Commit {
    fn eq(&self, other: &Self) -> bool {
        self.tree_id == other.tree_id
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

impl From<commit::Model> for Commit {
    fn from(value: commit::Model) -> Self {
        Commit {
            id: Hash::new_from_str(&value.git_id),
            tree_id: Hash::new_from_str(&value.tree),
            parent_tree_ids: value
                .pid
                .into_iter()
                .map(|id| Hash::new_from_str(&id))
                .collect(),
            author: Signature::new_from_data(value.author.unwrap().into()).unwrap(),
            committer: Signature::new_from_data(value.committer.unwrap().into()).unwrap(),
            message: value.content.unwrap(),
        }
    }
}

impl Commit {
    /// Create a new commit object from a meta object
    #[allow(unused)]
    pub fn new_from_meta(meta: Meta) -> Result<Commit, GitError> {
        Ok(Commit::new_from_data(meta.data))
    }

    // /// Create a new commit object from a file
    // #[allow(unused)]
    // pub fn new_from_file(path: &str) -> Result<Commit, GitError> {
    //     let meta = Meta::new_from_file(path)?;

    //     Commit::new_from_meta(meta)
    // }

    #[allow(unused)]
    pub fn to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend(b"tree ");
        data.extend(self.tree_id.to_plain_str().as_bytes());
        data.extend(&[0x0a]);

        for parent_tree_id in &self.parent_tree_ids {
            data.extend(b"parent ");
            data.extend(parent_tree_id.to_plain_str().as_bytes());
            data.extend(&[0x0a]);
        }

        data.extend(self.author.to_data()?);
        data.extend(&[0x0a]);
        data.extend(self.committer.to_data()?);
        data.extend(&[0x0a]);
        data.extend(self.message.as_bytes());

        Ok(data)
    }

    // #[allow(unused)]
    // pub fn to_file(&self, path: &str) -> Result<PathBuf, GitError> {
    //     self.meta.to_file(path)
    // }
}

impl ObjectT for Commit {
    fn get_hash(&self) -> Hash {
        self.id
    }

    fn get_raw(&self) -> Vec<u8> {
        self.to_data().unwrap()
    }
    fn get_type(&self) -> crate::internal::ObjectType {
        ObjectType::Commit
    }
    fn set_hash(&mut self, h: Hash) {
        self.id = h;
    }

    fn new_from_data(data: Vec<u8>) -> Self
    where
        Self: Sized,
    {
        let mut commit = data;
        // Find the tree id and remove it from the data
        let tree_end = commit.find_byte(0x0a).unwrap();
        let tree_id: Hash = Hash::new_from_str(
            String::from_utf8(commit[5..tree_end].to_owned())
                .unwrap()
                .as_str(),
        );
        commit = commit[tree_end + 1..].to_vec();

        // Find the parent tree ids and remove them from the data
        let author_begin = commit.find("author").unwrap();
        let parent_tree_ids: Vec<Hash> = commit[..author_begin]
            .find_iter("parent")
            .map(|parent| {
                let parent_end = commit[parent..].find_byte(0x0a).unwrap();
                Hash::new_from_str(
                    String::from_utf8(commit[parent + 7..parent + parent_end].to_owned())
                        .unwrap()
                        .as_str(),
                )
            })
            .collect();
        commit = commit[author_begin..].to_vec();

        // Find the author and committer and remove them from the data
        let author =
            Signature::new_from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec()).unwrap();
        commit = commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec();
        let committer =
            Signature::new_from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec()).unwrap();

        // The rest is the message
        let message = unsafe {
            String::from_utf8_unchecked(commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec())
        };

        Commit {
            id: Hash([0u8; 20]),
            tree_id,
            parent_tree_ids,
            author,
            committer,
            message,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::env;
    use std::path::PathBuf;

    use crate::internal::object::commit::Commit;
    use crate::internal::object::meta::Meta;
    use crate::internal::object::ObjectT;
    use crate::internal::ObjectType;

    // use crate::internal::object::meta::Meta;
    // use crate::internal::ObjectType;
    // use std::fs::remove_file;

    #[test]
    fn test_new_from_file_without_parent() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/c5/170dd0aae2dc2a9142add9bb24597d326714d7");

        let meta = Meta::new_from_file(source.to_str().unwrap()).unwrap();
        let commit = Commit::from_meta(meta);
        assert_eq!(
            commit.tree_id.to_plain_str(),
            "f9a1667a0dfce06819394c2aad557a04e9a13e56"
        );
        assert_eq!(
            commit.id.to_plain_str(),
            "c5170dd0aae2dc2a9142add9bb24597d326714d7"
        )
    }

    #[test]
    fn test_new_from_file_with_parent() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/4b/00093bee9b3ef5afc5f8e3645dc39cfa2f49aa");
        let meta = Meta::new_from_file(source.to_str().unwrap()).unwrap();
        let commit = Commit::from_meta(meta);
        assert_eq!(commit.parent_tree_ids.len(), 1);
        assert_eq!(
            commit.tree_id.to_plain_str(),
            "e7002dbbc79a209462247302c7757a31ab16df1e"
        );
        assert_eq!(
            commit.get_hash().to_plain_str(),
            "4b00093bee9b3ef5afc5f8e3645dc39cfa2f49aa"
        );
    }

    #[test]
    fn test_new_from_meta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/c5/170dd0aae2dc2a9142add9bb24597d326714d7");

        let meta = Meta::new_from_file(source.to_str().unwrap()).unwrap();
        let commit = Commit::from_meta(meta);
        assert_eq!(
            commit.id.to_plain_str(),
            "c5170dd0aae2dc2a9142add9bb24597d326714d7"
        );
        assert_eq!(commit.get_type(), ObjectType::Commit);
        assert_eq!(commit.author.name, "Quanyi Ma");
        println!("{}", commit)
    }

    #[test]
    fn test_new_from_data() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/4b/00093bee9b3ef5afc5f8e3645dc39cfa2f49aa");

        let meta = Meta::new_from_file(source.to_str().unwrap()).unwrap();
        let commit = Commit::from_meta(meta);

        assert_eq!(
            commit.tree_id.to_plain_str(),
            "e7002dbbc79a209462247302c7757a31ab16df1e"
        );
        assert_eq!(commit.get_type(), ObjectType::Commit);
        assert_eq!(commit.author.name, "Quanyi Ma");
    }

    // #[test]
    // fn test_to_file() {
    //     let source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
    //     let mut source_file = source.clone();
    //     let commit = Commit::new_from_file(source_file.to_str().unwrap()).unwrap();

    //     let mut dest_file = source.clone();
    //     dest_file.push("tests/objects/c5/170dd0aae2dc2a9142add9bb24597d326714d7");
    //     if dest_file.exists() {
    //         remove_file(dest_file.as_path().to_str().unwrap()).unwrap();
    //     }

    //     let mut dest = source.clone();
    //     dest = dest.join("tests");
    //     dest = dest.join("objects");

    //     // let file = commit.to_file(dest.to_str().unwrap()).unwrap();

    //     // assert_eq!(true, file.exists());
    // }
}
