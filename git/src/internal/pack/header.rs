use std::fmt::Display;

use crate::internal::pack::Hash;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone,Serialize, Deserialize,Default)]
pub enum EntryHeader {
    #[default]
    Commit,
    Tree,
    Blob,
    Tag,
    #[allow(unused)]
    RefDelta {
        base_id: Hash,
    },
    #[allow(unused)]
    OfsDelta {
        base_distance: usize,
    },
}

const COMMIT_OBJECT_TYPE: &[u8] = b"commit";
const TREE_OBJECT_TYPE: &[u8] = b"tree";
const BLOB_OBJECT_TYPE: &[u8] = b"blob";
const TAG_OBJECT_TYPE: &[u8] = b"tag";

impl EntryHeader {
    pub fn from_string(t: &str) -> Self {
        match t {
            "commit" => EntryHeader::Commit,
            "tree" => EntryHeader::Tree,
            "tag" => EntryHeader::Tag,
            "blob" => EntryHeader::Blob,
            _ => panic!("cat to not base obj"),
        }
    }
    pub fn is_base(&self) -> bool {
        match self {
            EntryHeader::Commit => true,
            EntryHeader::Tree => true,
            EntryHeader::Blob => true,
            EntryHeader::Tag => true,
            EntryHeader::RefDelta { base_id: _ } => false,
            EntryHeader::OfsDelta { base_distance: _ } => false,
        }
    }
    pub fn to_bytes(&self) -> &[u8] {
        match self {
            EntryHeader::Commit => COMMIT_OBJECT_TYPE,
            EntryHeader::Tree => TREE_OBJECT_TYPE,
            EntryHeader::Blob => BLOB_OBJECT_TYPE,
            EntryHeader::Tag => TAG_OBJECT_TYPE,
            _ => panic!("can put compute the delta hash value"),
        }
    }
    pub fn to_number(&self) -> u8{
        match self {
            EntryHeader::Commit => 1,
            EntryHeader::Tree => 2,
            EntryHeader::Blob => 3,
            EntryHeader::Tag => 4,
            EntryHeader::RefDelta { base_id:_ } => 7,
            EntryHeader::OfsDelta { base_distance:_ } => 6,
            
            
        }
    }
}
impl Display for EntryHeader{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            EntryHeader::Commit => write!(f, "COMMIT"),
            EntryHeader::Tree => write!(f, "Tree"),
            EntryHeader::Blob => write!(f, "Blob"),
            EntryHeader::Tag => write!(f, "Tag"),
            EntryHeader::RefDelta { base_id } =>write!(f, "Ref Delta :{}",base_id),
            EntryHeader::OfsDelta { base_distance } => write!(f, "Ofs Delta :{}",base_distance),
        }
        
    }
}