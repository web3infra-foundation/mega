//!
//! In the Git object binary model:
//!
//! - **Null** bytes are used as separators between the different fields to allow for efficient parsing
//! of the object.
//!
//!
//!
pub mod blob;
pub mod commit;
pub mod meta;
pub mod signature;
pub mod tag;
pub mod tree;

#[cfg(test)]
mod tests {}
