//! Mercury is a library for encode and decode Git Pack format file or stream.

// to avoid sticking on Dropping large HashMap on Windows
// but, mimalloc won't release memory to OS after dropping (TODO)
// see [issue](https://github.com/rust-lang/rust/issues/121747)
#[cfg(target_os = "windows")]
use mimalloc::MiMalloc;
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod internal;
pub mod hash;
pub mod errors;

#[cfg(test)]
mod tests {}
