//!
//! 
//! 
//! 
//! 
use std::io::{Read, Write};
use std::fmt::Display;

pub trait ObjectTrait: Read + Write + Send + Sync + Display {

}