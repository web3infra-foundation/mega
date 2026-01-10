#[macro_use]
extern crate log;

pub mod antares;
pub mod daemon;
pub mod dicfuse;
pub mod fuse;
pub mod manager;
pub mod server;
pub mod util;

//const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
const READONLY_INODE: u64 = 0xffff_ffff;
