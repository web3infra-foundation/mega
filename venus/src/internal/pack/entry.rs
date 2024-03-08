use callisto::{mega_blob, mega_commit, mega_tag, mega_tree};
use serde::{Deserialize, Serialize};

use crate::hash::SHA1;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tag::Tag;
use crate::internal::object::tree::Tree;
use crate::internal::object::types::{MegaModel, ObjectType, RawObject};
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
    pub fn convert_to_mega_model(self, mr_id: &str) -> MegaModel {
        match self {
            RawObject::Commit(commit) => {
                let mut mega_commit: mega_commit::Model = commit.into();
                mega_commit.mr_id = mr_id.to_owned();
                MegaModel::Commit(mega_commit.into())
            }
            RawObject::Tree(tree) => {
                let mut mega_tree: mega_tree::Model = tree.into();
                mega_tree.mr_id = mr_id.to_owned();
                MegaModel::Tree(mega_tree.into())
            }
            RawObject::Blob(blob) => {
                let mut mega_blob: mega_blob::Model = blob.into();
                mega_blob.mr_id = mr_id.to_owned();
                MegaModel::Blob(mega_blob.into())
            }
            RawObject::Tag(tag) => {
                let mega_tag: mega_tag::Model = tag.into();
                MegaModel::Tag(mega_tag.into())
            }
        }
    }
}
