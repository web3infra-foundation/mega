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

use sha1::Digest;

use crate::internal::model::sea_models::{
    git_blob as sea_git_blob, git_commit as sea_git_commit, git_tag as sea_git_tag,
    git_tree as sea_git_tree, mega_blob as sea_mega_blob, mega_commit as sea_mega_commit,
    mega_tag as sea_mega_tag, mega_tree as sea_mega_tree, raw_blob as sea_raw_blob,
};

use crate::internal::object::types::ObjectType;
use crate::internal::object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree};
use crate::internal::zlib::stream::inflate::ReadBoxed;
use crate::{errors::GitError, hash::SHA1};

pub trait ObjectTrait: Send + Sync + Display {
    /// Creates a new object from a byte slice.
    fn from_bytes(data: &[u8], hash: SHA1) -> Result<Self, GitError>
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
        Self::from_bytes(&content, SHA1::from_str(&format!("{hash_str:x}")).unwrap()).unwrap()
    }

    /// Returns the type of the object.
    fn get_type(&self) -> ObjectType;

    fn get_size(&self) -> usize;

    fn to_data(&self) -> Result<Vec<u8>, GitError>;
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
    Commit(sea_git_commit::Model),
    Tree(sea_git_tree::Model),
    Blob(sea_git_blob::Model, sea_raw_blob::Model),
    Tag(sea_git_tag::Model),
}

pub enum MegaObjectModel {
    Commit(sea_mega_commit::Model),
    Tree(sea_mega_tree::Model),
    Blob(sea_mega_blob::Model, sea_raw_blob::Model),
    Tag(sea_mega_tag::Model),
}

impl GitObject {
    pub fn convert_to_mega_model(self) -> MegaObjectModel {
        match self {
            GitObject::Commit(commit) => MegaObjectModel::Commit(commit.into()),
            GitObject::Tree(tree) => MegaObjectModel::Tree(tree.into()),
            GitObject::Blob(blob) => MegaObjectModel::Blob((&blob).into(), (&blob).into()),
            GitObject::Tag(tag) => MegaObjectModel::Tag(tag.into()),
        }
    }

    pub fn convert_to_git_model(self) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => GitObjectModel::Commit(commit.into()),
            GitObject::Tree(tree) => GitObjectModel::Tree(tree.into()),
            GitObject::Blob(blob) => GitObjectModel::Blob((&blob).into(), (&blob).into()),
            GitObject::Tag(tag) => GitObjectModel::Tag(tag.into()),
        }
    }
}
