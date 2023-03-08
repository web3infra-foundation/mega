//!
//!
//!
//!
//!
use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::signature::Signature;

#[allow(unused)]
pub struct Commit {
    pub meta: Meta,
    pub tree_id: Hash,
    pub parent_tree_ids: Vec<Hash>,
    pub author: Signature,
    pub committer: Signature,
    pub message: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}