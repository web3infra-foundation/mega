//!
//! 
//! 
//! 
//! 
pub mod types;
pub mod blob;

use std::fmt::Display;

use crate::internal::object::types::ObjectType;

pub trait ObjectTrait: Send + Sync + Display {
    fn from_bytes(data: Vec<u8>) -> Self where Self: Sized;

    fn get_type(&self) -> ObjectType;
}