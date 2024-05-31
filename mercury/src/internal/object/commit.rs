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
    pub id: SHA1,
    pub tree_id: SHA1,
    pub parent_commit_ids: Vec<SHA1>,
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
        for parent in self.parent_commit_ids.iter() {
            writeln!(f, "parent: {}", parent)?;
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
        let hash = SHA1::from_type_and_data(ObjectType::Commit, &commit.to_data().unwrap());
        commit.id = hash;
        commit
    }

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
}

impl ObjectTrait for Commit {
    fn from_bytes(data: &[u8], hash: SHA1) -> Result<Self, GitError>
    where
        Self: Sized,
    {
        let commit = data;

        // Find the tree id
        let tree_end = commit
            .iter()
            .position(|&b| b == 0x0a)
            .ok_or(GitError::InvalidCommitObject)?;
        let tree_id = SHA1::from_str(
            std::str::from_utf8(&commit[5..tree_end]).map_err(|_| GitError::InvalidCommitObject)?,
        )
        .unwrap();

        // Find the parent commit ids
        let author_begin = commit
            .windows(6)
            .position(|window| window == b"author")
            .ok_or(GitError::InvalidCommitObject)?;
        let parent_commit_ids = commit[..author_begin]
            .windows(6)
            .enumerate()
            .filter_map(|(i, window)| {
                if window == b"parent" {
                    let parent_end = commit[i + 7..].iter().position(|&b| b == 0x0a)?;
                    Some(
                        SHA1::from_str(
                            std::str::from_utf8(&commit[i + 7..i + 7 + parent_end]).ok()?,
                        )
                        .ok(),
                    )
                } else {
                    None
                }
            })
            .collect::<Option<Vec<SHA1>>>()
            .ok_or(GitError::InvalidCommitObject)?;

        // Extract author
        let author_end = commit[author_begin..]
            .iter()
            .position(|&b| b == 0x0a)
            .ok_or(GitError::InvalidCommitObject)?
            + author_begin;
        let author = Signature::from_data(commit[author_begin..author_end].to_vec())?;

        // Extract committer
        let committer_begin = author_end + 1;
        let committer_end = commit[committer_begin..]
            .iter()
            .position(|&b| b == 0x0a)
            .ok_or(GitError::InvalidCommitObject)?
            + committer_begin;
        let committer = Signature::from_data(commit[committer_begin..committer_end].to_vec())?;

        // The rest is the message
        let message = std::str::from_utf8(&commit[committer_end + 1..])
            .map_err(|_| GitError::InvalidCommitObject)?
            .to_string();

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
        data.extend(self.tree_id.to_plain_str().as_bytes());
        data.extend(&[0x0a]);

        for parent_tree_id in &self.parent_commit_ids {
            data.extend(b"parent ");
            data.extend(parent_tree_id.to_plain_str().as_bytes());
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
