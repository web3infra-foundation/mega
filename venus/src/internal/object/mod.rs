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
    Commit(mega_commit::Model),
    Tree(mega_tree::Model),
    Blob(mega_blob::Model, raw_blob::Model),
    Tag(mega_tag::Model),
}

impl GitObject {
    pub fn convert_to_mega_model(self) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => {
                let mega_commit: mega_commit::Model = commit.into();
                GitObjectModel::Commit(mega_commit)
            }
            GitObject::Tree(tree) => {
                let mega_tree: mega_tree::Model = tree.into();
                GitObjectModel::Tree(mega_tree)
            }
            GitObject::Blob(blob) => {
                let mega_blob: mega_blob::Model = blob.clone().into();
                let raw_blob: raw_blob::Model = blob.into();
                GitObjectModel::Blob(mega_blob, raw_blob)
            }
            GitObject::Tag(tag) => {
                let mega_tag: mega_tag::Model = tag.into();
                GitObjectModel::Tag(mega_tag)
            }
        }
    }
}
