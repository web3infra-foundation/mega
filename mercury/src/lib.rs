//! Mercury is a library for encode and decode Git Pack format file or stream.

// to avoid sticking on Dropping large HashMap on Windows
// but, mimalloc won't release memory to OS after dropping (TODO)
// see [issue](https://github.com/rust-lang/rust/issues/121747)
#[cfg(target_os = "windows")]
use mimalloc::MiMalloc;
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(not(target_os = "windows"))]
pub(crate) const MERCURY_DEFAULT_TMP_DIR: &str = "/tmp/.cache_temp";
#[cfg(target_os = "windows")]
pub(crate) const MERCURY_DEFAULT_TMP_DIR: &str = "%TEMP%\\.cache_temp";

pub mod internal;
pub mod hash;
pub mod errors;
pub mod utils;

#[cfg(test)]
mod tests {}
