//! The Commit object is a data structure used to represent a specific version of a project's
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
use std::fmt::Display;
use std::str::FromStr;

use crate::errors::GitError;
use crate::hash::SHA1;
use crate::internal::object::signature::Signature;
use crate::internal::object::ObjectTrait;
use crate::internal::object::ObjectType;
use bincode::{Decode, Encode};
use bstr::ByteSlice;
use callisto::git_commit;
use callisto::mega_commit;
use serde::Deserialize;
use serde::Serialize;

/// The `Commit` struct is used to represent a commit object.
///
/// - The tree object SHA points to the top level tree for this commit, which reflects the complete
///   state of the repository at the time of the commit. The tree object in turn points to blobs and
///   subtrees which represent the files in the repository.
/// - The parent commit SHAs allow Git to construct a linked list of commits and build the full
///   commit history. By chaining together commits in this fashion, Git is able to represent the entire
///   history of a repository with a single commit object at its root.
/// - The author and committer fields contain the name, email address, timestamp and timezone.
/// - The message field contains the commit message, which maybe include signed or DCO.
#[derive(Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
#[non_exhaustive]
pub struct Commit {
    pub id: SHA1,
    pub tree_id: SHA1,
    pub parent_commit_ids: Vec<SHA1>,
    pub author: Signature,
    pub committer: Signature,
    pub message: String,
}
impl PartialEq for Commit {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "tree: {}", self.tree_id)?;
        for parent in self.parent_commit_ids.iter() {
            writeln!(f, "parent: {parent}")?;
        }
        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;
        writeln!(f, "{}", self.message)
    }
}

impl Commit {
    pub fn new(
        author: Signature,
        committer: Signature,
        tree_id: SHA1,
        parent_commit_ids: Vec<SHA1>,
        message: &str,
    ) -> Commit {
        let mut commit = Commit {
            id: SHA1::default(),
            tree_id,
            parent_commit_ids,
            author,
            committer,
            message: message.to_string(),
        };
        // Calculate the hash of the commit object
        // The hash is calculated from the type and data of the commit object
        let hash = SHA1::from_type_and_data(ObjectType::Commit, &commit.to_data().unwrap());
        commit.id = hash;
        commit
    }

    /// Creates a new commit object from a tree ID and a list of parent commit IDs.
    /// This function generates the author and committer signatures using the current time
    /// and a fixed email address.
    /// It also sets the commit message to the provided string.
    /// # Arguments
    /// - `tree_id`: The SHA1 hash of the tree object that this commit points to.
    /// - `parent_commit_ids`: A vector of SHA1 hashes of the parent commits.
    /// - `message`: A string containing the commit message.
    /// # Returns
    /// A new `Commit` object with the specified tree ID, parent commit IDs, and commit message.
    /// The author and committer signatures are generated using the current time and a fixed email address.
    pub fn from_tree_id(tree_id: SHA1, parent_commit_ids: Vec<SHA1>, message: &str) -> Commit {
        let author = Signature::from_data(
            format!(
                "author mega <admin@mega.org> {} +0800",
                chrono::Utc::now().timestamp()
            )
            .to_string()
            .into_bytes(),
        )
        .unwrap();
        let committer = Signature::from_data(
            format!(
                "committer mega <admin@mega.org> {} +0800",
                chrono::Utc::now().timestamp()
            )
            .to_string()
            .into_bytes(),
        )
        .unwrap();
        Commit::new(author, committer, tree_id, parent_commit_ids, message)
    }

    /// Formats the commit message by extracting the first line of the message.
    /// If the message contains a PGP signature, it will return the first line after the signature.
    pub fn format_message(&self) -> String {
        let mut has_signature = false;
        for line in self.message.lines() {
            if has_signature && !line.trim().is_empty() {
                return line.to_owned();
            }
            if line.contains("-----END PGP SIGNATURE-----") {
                has_signature = true;
            }
        }
        // does not have pgp, find first line has data
        for line in self.message.lines() {
            if !line.trim().is_empty() {
                return line.to_owned();
            }
        }
        self.message.clone()
    }
}

impl ObjectTrait for Commit {
    fn from_bytes(data: &[u8], hash: SHA1) -> Result<Self, GitError>
    where
        Self: Sized,
    {
        let mut commit = data;
        // Find the tree id and remove it from the data
        let tree_end = commit.find_byte(0x0a).unwrap();
        let tree_id: SHA1 = SHA1::from_str(
            String::from_utf8(commit[5..tree_end].to_owned()) // 5 is the length of "tree "
                .unwrap()
                .as_str(),
        )
        .unwrap();
        let binding = commit[tree_end + 1..].to_vec(); // Move past the tree id
        commit = &binding;

        // Find the parent commit ids and remove them from the data
        let author_begin = commit.find("author").unwrap();
        // Find all parent commit ids
        // The parent commit ids are all the lines that start with "parent "
        // We can use find_iter to find all occurrences of "parent "
        // and then extract the SHA1 hashes from them.
        let parent_commit_ids: Vec<SHA1> = commit[..author_begin]
            .find_iter("parent")
            .map(|parent| {
                let parent_end = commit[parent..].find_byte(0x0a).unwrap();
                SHA1::from_str(
                    // 7 is the length of "parent "
                    String::from_utf8(commit[parent + 7..parent + parent_end].to_owned())
                        .unwrap()
                        .as_str(),
                )
                .unwrap()
            })
            .collect();
        let binding = commit[author_begin..].to_vec();
        commit = &binding;

        // Find the author and committer and remove them from the data
        // 0x0a is the newline character
        let author =
            Signature::from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec()).unwrap();

        let binding = commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec();
        commit = &binding;
        let committer =
            Signature::from_data(commit[..commit.find_byte(0x0a).unwrap()].to_vec()).unwrap();

        // The rest is the message
        let message = unsafe {
            String::from_utf8_unchecked(commit[commit.find_byte(0x0a).unwrap() + 1..].to_vec())
        };

        Ok(Commit {
            id: hash,
            tree_id,
            parent_commit_ids,
            author,
            committer,
            message,
        })
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::Commit
    }

    fn get_size(&self) -> usize {
        0
    }

    /// [Git-Internals-Git-Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)
    fn to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend(b"tree ");
        data.extend(self.tree_id.to_string().as_bytes());
        data.extend(&[0x0a]);

        for parent_tree_id in &self.parent_commit_ids {
            data.extend(b"parent ");
            data.extend(parent_tree_id.to_string().as_bytes());
            data.extend(&[0x0a]);
        }

        data.extend(self.author.to_data()?);
        data.extend(&[0x0a]);
        data.extend(self.committer.to_data()?);
        data.extend(&[0x0a]);
        // Important! or Git Server can't parse & reply: unpack-objects abnormal exit
        // We can move [0x0a] to message instead here.
        // data.extend(&[0x0a]);
        data.extend(self.message.as_bytes());

        Ok(data)
    }
}
fn commit_from_model(
    commit_id: &str,
    tree: &str,
    parents_id: &serde_json::Value,
    author: Option<String>,
    committer: Option<String>,
    message: Option<String>,
) -> Commit {
    Commit {
        id: SHA1::from_str(commit_id).unwrap(),
        tree_id: SHA1::from_str(tree).unwrap(),
        parent_commit_ids: parents_id
            .as_array()
            .unwrap()
            .iter()
            .map(|id| SHA1::from_str(id.as_str().unwrap()).unwrap())
            .collect(),
        author: Signature::from_data(author.unwrap().into()).unwrap(),
        committer: Signature::from_data(committer.unwrap().into()).unwrap(),
        message: message.unwrap(),
    }
}

impl From<mega_commit::Model> for Commit {
    fn from(value: mega_commit::Model) -> Self {
        commit_from_model(
            &value.commit_id,
            &value.tree,
            &value.parents_id,
            value.author,
            value.committer,
            value.content,
        )
    }
}

impl From<git_commit::Model> for Commit {
    fn from(value: git_commit::Model) -> Self {
        commit_from_model(
            &value.commit_id,
            &value.tree,
            &value.parents_id,
            value.author,
            value.committer,
            value.content,
        )
    }
}
