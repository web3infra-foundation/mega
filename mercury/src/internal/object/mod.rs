//!
//! 
//! 
//! 
//! 
pub mod types;
pub mod blob;

use std::fmt::Display;

use crate::hash::SHA1;
use crate::internal::object::types::ObjectType;

pub trait ObjectTrait: Send + Sync + Display {
    /// Creates a new object from a byte slice.
    fn from_bytes(data: Vec<u8>) -> Self where Self: Sized;

    /// Creates a new object from a byte slice with a given ID.
    fn from_bytes_with_id(data: Vec<u8>, id: SHA1) -> Self where Self: Sized;

    /// Returns the type of the object.
    fn get_type(&self) -> ObjectType;
}