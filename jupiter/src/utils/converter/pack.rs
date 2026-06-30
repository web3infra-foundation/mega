use git_internal::internal::{
    object::{ObjectTrait, blob::Blob, commit::Commit, tag::Tag, tree::Tree, types::ObjectType},
    pack::entry::Entry,
};

use super::traits::GitObject;

pub fn process_entry(entry: Entry) -> GitObject {
    match entry.obj_type {
        ObjectType::Commit => {
            GitObject::Commit(Commit::from_bytes(&entry.data, entry.hash).unwrap())
        }
        ObjectType::Tree => GitObject::Tree(Tree::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Blob => GitObject::Blob(Blob::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Tag => GitObject::Tag(Tag::from_bytes(&entry.data, entry.hash).unwrap()),
        _ => unreachable!("can not parse delta!"),
    }
}
