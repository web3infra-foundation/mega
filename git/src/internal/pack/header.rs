
use crate::internal::pack::Hash;
#[derive(Debug, Clone)]
pub enum EntryHeader {
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
}
