//! general tool.
//!
//! This module contains common tools for the program, such as: environment
//! variables, task generation macros.

mod env;
pub mod file;
mod parser;

pub use self::env::EnvVar;
pub use self::parser::Parser;
