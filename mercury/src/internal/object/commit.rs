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
use crate::internal::model::sea_models::{
    git_commit as sea_git_commit, mega_commit as sea_mega_commit,
};
use crate::internal::object::signature::Signature;
use crate::internal::object::ObjectTrait;
use crate::internal::object::ObjectType;
use bincode::{Decode, Encode};
use bstr::ByteSlice;
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
    parents_id: &str,
    author: Option<String>,
    committer: Option<String>,
    message: Option<String>,
) -> Commit {
    Commit {
        id: SHA1::from_str(commit_id).unwrap(),
        tree_id: SHA1::from_str(tree).unwrap(),
        parent_commit_ids: serde_json::from_str::<Vec<String>>(parents_id)
            .unwrap()
            .iter()
            .map(|id| SHA1::from_str(id).unwrap())
            .collect(),
        author: Signature::from_data(author.unwrap().into()).unwrap(),
        committer: Signature::from_data(committer.unwrap().into()).unwrap(),
        message: message.unwrap(),
    }
}

impl From<sea_mega_commit::Model> for Commit {
    fn from(value: sea_mega_commit::Model) -> Self {
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

impl From<sea_git_commit::Model> for Commit {
    fn from(value: sea_git_commit::Model) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_from_bytes_with_gpgsig() {
        let raw_commit = br#"tree 341e54913a3a43069f2927cc0f703e5a9f730df1
author benjamin.747 <benjamin.747@outlook.com> 1757467768 +0800
committer benjamin.747 <benjamin.747@outlook.com> 1757491219 +0800
gpgsig -----BEGIN PGP SIGNATURE-----
 
 iQJNBAABCAA3FiEEs4MaYUV7JcjxsVMPyqxGczTZ6K4FAmjBMC4ZHGJlbmphbWlu
 Ljc0N0BvdXRsb29rLmNvbQAKCRDKrEZzNNnorj73EADNpsyLAHsB3NgoeH+uy9Vq
 G2+LRtlvqv3QMK7vbQUadXHlQYWk25SIk+WJ1kG1AnUy5fqOrLSDTA1ny+qwpH8O
 +2sKCF/S1wlzqGWjCcRH5/ir9srsGIn9HbNqBjmU22NJ6Dt2jnqoUvtWfPwyqwWg
 VpjYlj390cFdXTpH5hMvtlmUQB+zCSKtWQW2Ur64h/UsGtllARlACi+KHQQmA2/p
 FLWNddvfJQpPM597DkGohQTD68g0PqOBhUkOHduHq7VHy68DVW+07bPNXK8JhJ8S
 4dyV1sZwcVcov0GcKl0wUbEqzy4gf+zV7DQhkfrSRQMBdo5vCWahYj1AbgaTiu8a
 hscshYDuWWqpxBU/+nCxOPskV29uUG1sRyXp3DqmKJZpnO9CVdw3QaVrqnMEeh2S
 t/wYRI9aI1A+Mi/DETom5ifTVygMkK+3m1h7pAMOlblFEdZx2sDXPRG2IEUcatr4
 Jb2+7PUJQXxUQnwHC7xHHxRh6a2h8TfEJfSoEyrgzxZ0CRxJ6XMJaJu0UwZ2xMsx
 Lgmeu6miB/imwxz5R5RL2yVHbgllSlO5l12AIeBaPoarKXYPSALigQnKCXu5OM3x
 Jq5qsSGtxdr6S1VgLyYHR4o69bQjzBp9K47J3IXqvrpo/ZiO/6Mspk2ZRWhGj82q
 e3qERPp5b7+hA+M7jKPyJg==
 =UeLf
 -----END PGP SIGNATURE-----

test parse commit from bytes
"#;

        let hash = SHA1::from_str("57d7685c60213a9da465cf900f31933be3a7ee39").unwrap();
        let commit = Commit::from_bytes(raw_commit, hash).unwrap();

        assert_eq!(
            commit.id,
            SHA1::from_str("57d7685c60213a9da465cf900f31933be3a7ee39").unwrap()
        );

        assert_eq!(
            commit.tree_id,
            SHA1::from_str("341e54913a3a43069f2927cc0f703e5a9f730df1").unwrap()
        );

        assert_eq!(commit.author.name, "benjamin.747");
        assert_eq!(commit.author.email, "benjamin.747@outlook.com");

        assert_eq!(commit.committer.name, "benjamin.747");

        // check message content（must contains gpgsig and content）
        assert!(commit.message.contains("-----BEGIN PGP SIGNATURE-----"));
        assert!(commit.message.contains("-----END PGP SIGNATURE-----"));
        assert!(commit.message.contains("test parse commit from bytes"));
    }
}
