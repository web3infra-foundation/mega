//!
//! 
//! 
//! 
//! 
pub mod types;
pub mod blob;

use std::io::{Read, Write};
use std::fmt::Display;


/// Implementation of the `Object` trait.
///
/// The naming conventions for the methods in this implementation are designed to be intuitive and self-explanatory:
///
/// 1. `new` Prefix: 
///
/// 2. `from` Prefix:
///
/// 3. `to` Prefix:
///
/// These method naming conventions (`new`, `from`, `to`) provide clarity and predictability in the API, making it easier for users 
/// to understand the intended use and functionality of each method within the `SHA1` struct.
pub trait ObjectTrait: Read + Write + Send + Sync + Display {
    fn new(data: impl Read) -> Self;
}