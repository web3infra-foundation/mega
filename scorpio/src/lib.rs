
#[macro_use]
extern crate log;


pub mod fuse;
mod dicfuse;
pub mod util;
pub mod manager;
pub mod server;
pub mod daemon;
mod scolfs;
//const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
const READONLY_INODE :u64 = 0xffff_ffff;
