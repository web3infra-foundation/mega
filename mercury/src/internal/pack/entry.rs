use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::hash::SHA1;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tag::Tag;
use crate::internal::object::tree::Tree;
use crate::internal::object::types::ObjectType;
use crate::internal::object::{GitObject, ObjectTrait};

///
/// Git object data from pack file
///
#[derive(Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub obj_type: ObjectType,
    pub data: Vec<u8>,
    pub hash: SHA1,
    pub chain_len: usize,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        // hash is enough to compare, right?
        self.obj_type == other.obj_type && self.hash == other.hash
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.obj_type.hash(state);
        self.hash.hash(state);
    }
}

impl Entry {
    pub fn process_entry(&self) -> GitObject {
        match self.obj_type {
            ObjectType::Commit => {
                GitObject::Commit(Commit::from_bytes(&self.data, self.hash).unwrap())
            }
            ObjectType::Tree => GitObject::Tree(Tree::from_bytes(&self.data, self.hash).unwrap()),
            ObjectType::Blob => GitObject::Blob(Blob::from_bytes(&self.data, self.hash).unwrap()),
            ObjectType::Tag => GitObject::Tag(Tag::from_bytes(&self.data, self.hash).unwrap()),
            _ => unreachable!("can not parse delta!"),
        }
    }
}

impl From<Blob> for Entry {
    fn from(value: Blob) -> Self {
        Self {
            obj_type: ObjectType::Blob,
            data: value.data,
            hash: value.id,
            chain_len: 0,
        }
    }
}

impl From<Commit> for Entry {
    fn from(value: Commit) -> Self {
        Self {
            obj_type: ObjectType::Commit,
            data: value.to_data().unwrap(),
            hash: value.id,
            chain_len: 0,
        }
    }
}

impl From<Tree> for Entry {
    fn from(value: Tree) -> Self {
        Self {
            obj_type: ObjectType::Tree,
            data: value.to_data().unwrap(),
            hash: value.id,
            chain_len: 0,
        }
    }
}

impl From<Tag> for Entry {
    fn from(value: Tag) -> Self {
        Self {
            obj_type: ObjectType::Tag,
            data: value.to_data().unwrap(),
            hash: value.id,
            chain_len: 0,
        }
    }
}
