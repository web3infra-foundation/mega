pub mod blob;
pub mod commit;
pub mod signature;
pub mod tag;
pub mod tree;
pub mod types;
pub mod utils;

use std::{
    fmt::Display,
    io::{BufRead, Read},
    str::FromStr,
};

use callisto::{mega_blob, mega_commit, mega_tag, mega_tree, raw_blob};
use sha1::Digest;

use crate::internal::object::types::ObjectType;
use crate::internal::object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree};
use crate::internal::zlib::stream::inflate::ReadBoxed;
use crate::{errors::GitError, hash::SHA1};

pub trait ObjectTrait: Send + Sync + Display {
    /// Creates a new object from a byte slice.
    fn from_bytes(data: Vec<u8>, hash: SHA1) -> Result<Self, GitError>
    where
        Self: Sized;

    /// Generate a new Object from a `ReadBoxed<BufRead>`.
    /// the input size,is only for new a vec with directive space allocation
    /// the input data stream and output object should be plain base object .
    fn from_buf_read<R: BufRead>(read: &mut ReadBoxed<R>, size: usize) -> Self
    where
        Self: Sized,
    {
        let mut content: Vec<u8> = Vec::with_capacity(size);
        read.read_to_end(&mut content).unwrap();
        let h = read.hash.clone();
        let hash_str = h.finalize();
        Self::from_bytes(content, SHA1::from_str(&format!("{:x}", hash_str)).unwrap()).unwrap()
    }

    /// Returns the type of the object.
    fn get_type(&self) -> ObjectType;

    ///
    fn get_size(&self) -> usize;
}

#[derive(PartialEq, Debug, Clone)]
pub enum GitObject {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
    Tag(Tag),
}

#[derive(PartialEq, Debug, Clone)]
pub enum GitObjectModel {
    Commit(mega_commit::ActiveModel),
    Tree(mega_tree::ActiveModel),
    Blob(mega_blob::ActiveModel, raw_blob::ActiveModel),
    Tag(mega_tag::ActiveModel),
}

impl GitObject {
    pub fn convert_to_mega_model(self, repo_id: i64, mr_id: i64) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => {
                let mut mega_commit: mega_commit::Model = commit.into();
                mega_commit.mr_id = mr_id;
                mega_commit.repo_id = repo_id;
                GitObjectModel::Commit(mega_commit.into())
            }
            GitObject::Tree(tree) => {
                let mut mega_tree: mega_tree::Model = tree.into();
                mega_tree.mr_id = mr_id;
                mega_tree.repo_id = repo_id;
                GitObjectModel::Tree(mega_tree.into())
            }
            GitObject::Blob(blob) => {
                let mut mega_blob: mega_blob::Model = blob.clone().into();
                let raw_blob: raw_blob::Model = blob.into();
                mega_blob.mr_id = mr_id;
                mega_blob.repo_id = repo_id;
                GitObjectModel::Blob(mega_blob.into(), raw_blob.into())
            }
            GitObject::Tag(tag) => {
                let mut mega_tag: mega_tag::Model = tag.into();
                mega_tag.repo_id = repo_id;
                GitObjectModel::Tag(mega_tag.into())
            }
        }
    }
}
