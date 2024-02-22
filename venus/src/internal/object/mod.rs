pub mod blob;
pub mod commit;
pub mod signature;
pub mod tree;
pub mod types;
pub mod utils;

use std::fmt::Display;

use crate::errors::GitError;
use crate::internal::object::types::ObjectType;

pub trait ObjectTrait: Send + Sync + Display {
    /// Creates a new object from a byte slice.
    fn from_bytes(data: Vec<u8>) -> Result<Self, GitError>
    where
        Self: Sized;

    /// Returns the type of the object.
    fn get_type(&self) -> ObjectType;

    ///
    fn get_size(&self) -> usize;
}
