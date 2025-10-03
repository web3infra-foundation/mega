#[macro_use]
extern crate log;

pub mod daemon;
pub mod dicfuse;
pub mod fuse;
pub mod internal;
pub mod manager;
mod scolfs;
pub mod server;
pub mod util;
pub mod utils;
//const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
const READONLY_INODE: u64 = 0xffff_ffff;
