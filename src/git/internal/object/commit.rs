//!
//!
//!
//!
//!


use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::signature::AuthorSignature;

#[allow(unused)]
pub struct Commit {
    pub meta: Meta,
    pub tree_id: Hash,
    pub parent_tree_ids: Vec<Hash>,
    pub author: AuthorSignature,
    pub committer: AuthorSignature,
    pub message: String,
}

mod tests {

}