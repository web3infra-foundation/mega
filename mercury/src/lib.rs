//! Mercury is a library for encoding and decoding Git Pack format files or streams.

pub mod internal;
pub mod hash;
pub mod errors;
pub mod utils;

#[cfg(test)]
mod tests {}
