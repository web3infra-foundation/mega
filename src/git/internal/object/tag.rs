//!
//!
//!
//!
//!

use crate::git::hash::Hash;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::signature::AuthorSignature;
use crate::git::internal::ObjectType;

#[allow(unused)]
pub struct Tag {
    pub meta: Meta,
    pub object: Hash,
    pub object_type: ObjectType,
    pub tag: String,
    pub tagger: AuthorSignature,
    pub message: String,
}

#[cfg(test)]
mod tests {

}