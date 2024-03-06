//! Mercury is a library for encode and decode Git Pack format file or stream.
//!
//!
//!

// to avoid sticking on Dropping large HashMap
// see [issue](https://github.com/rust-lang/rust/issues/121747)
use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod cache;
pub mod internal;

#[cfg(test)]
mod tests {}
