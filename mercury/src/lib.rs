//! Mercury is a library for encoding and decoding Git Pack format files or streams.

pub mod errors;
pub mod hash;
pub mod internal;
pub mod utils;

#[cfg(test)]
mod tests {}
