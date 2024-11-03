

#[macro_use]
extern crate log;
mod passthrough;
mod overlayfs;
//mod store;
// pub mod fuse;
mod dicfuse;
mod util;
// pub mod manager;
pub mod server;
// pub mod deamon;
//const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
pub const READONLY_INODE :u64 = 0xffff_ffff;
