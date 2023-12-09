//!
//! 
//! 
//! 
//! 
pub mod types;
pub mod blob;

use std::io::Read;
use std::fmt::Display;

pub trait ObjectTrait: Send + Sync + Display {
    fn new(&self, data: impl Read) -> Self where Self: Sized;
}