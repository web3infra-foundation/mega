 
#[allow(unused)]
const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
#[allow(unused)]
const READONLY_INODE :u64 = 0xffff_ffff;
// Alloc inode numbers at one batch
#[allow(unused)]
const INODE_ALLOC_BATCH:u64 = 0xf_0000_0000;