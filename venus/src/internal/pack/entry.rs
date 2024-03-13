use callisto::{mega_blob, mega_commit, mega_tag, mega_tree, raw_blob};
use serde::{Deserialize, Serialize};

use crate::hash::SHA1;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tag::Tag;
use crate::internal::object::tree::Tree;
use crate::internal::object::types::ObjectType;
use crate::internal::object::ObjectTrait;

///
/// Git object for storage
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub obj_type: ObjectType,
    pub data: Vec<u8>,
    pub hash: SHA1,
}

#[derive(PartialEq, Debug, Clone)]
pub enum RawObject {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
    Tag(Tag),
}

#[derive(PartialEq, Debug, Clone)]
pub enum MegaModel {
    Commit(mega_commit::ActiveModel),
    Tree(mega_tree::ActiveModel),
    Blob(mega_blob::ActiveModel, raw_blob::ActiveModel),
    Tag(mega_tag::ActiveModel),
}

impl Entry {
    pub fn process_entry(&self) -> RawObject {
        match self.obj_type {
            ObjectType::Commit => {
                RawObject::Commit(Commit::from_bytes(self.data.clone(), self.hash).unwrap())
            }
            ObjectType::Tree => {
                RawObject::Tree(Tree::from_bytes(self.data.clone(), self.hash).unwrap())
            }
            ObjectType::Blob => {
                RawObject::Blob(Blob::from_bytes(self.data.clone(), self.hash).unwrap())
            }
            ObjectType::Tag => {
                RawObject::Tag(Tag::from_bytes(self.data.clone(), self.hash).unwrap())
            }
            _ => unreachable!("can not parse delta!"),
        }
    }
}

impl RawObject {
    pub fn convert_to_mega_model(self, repo_id: i64, mr_id: i64) -> MegaModel {
        match self {
            RawObject::Commit(commit) => {
                let mut mega_commit: mega_commit::Model = commit.into();
                mega_commit.mr_id = mr_id;
                mega_commit.repo_id = repo_id;
                MegaModel::Commit(mega_commit.into())
            }
            RawObject::Tree(tree) => {
                let mut mega_tree: mega_tree::Model = tree.into();
                mega_tree.mr_id = mr_id;
                mega_tree.repo_id = repo_id;
                MegaModel::Tree(mega_tree.into())
            }
            RawObject::Blob(blob) => {
                let mut mega_blob: mega_blob::Model = blob.clone().into();
                let raw_blob: raw_blob::Model = blob.into();
                mega_blob.mr_id = mr_id;
                mega_blob.repo_id = repo_id;
                MegaModel::Blob(mega_blob.into(), raw_blob.into())
            }
            RawObject::Tag(tag) => {
                let mut mega_tag: mega_tag::Model = tag.into();
                mega_tag.repo_id = repo_id;
                MegaModel::Tag(mega_tag.into())
            }
        }
    }
}

 
impl From<Blob> for Entry {
    fn from(value: Blob) -> Self {
        Self {
            obj_type: ObjectType::Blob,
            data: value.data,
            hash: value.id,
        }
    }
}