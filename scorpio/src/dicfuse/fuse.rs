use std::time::Duration;

use libc::stat64;
use fuse_backend_rs::{abi::fuse_abi::Attr, api::filesystem::Entry};
const BLOCK_SIZE: u32 = 512;
fn default_stat64(inode:u64) -> stat64 {
    let t = Attr{
        ino: inode,                       // Default inode number
        size: 512 ,                      // Default file size
        blocks: 8,                    // Default number of blocks
        atime: 0,                     // Default last access time
        mtime: 0,                     // Default last modification time
        ctime: 0,                     // Default last status change time
        atimensec: 0,                 // Default nanoseconds of last access time
        mtimensec: 0,                 // Default nanoseconds of last modification time
        ctimensec: 0,                 // Default nanoseconds of last status change time
        mode: 0o100444,                  // Default file mode (r--r--r--)
        //mode: 0o0040755,                  // Default file mode (r-xr-xr-x)
        nlink: 2,                     // Default number of hard links
        uid: 1000,                    // Default user ID
        gid: 1000,                    // Default group ID
        rdev: 0,                      // Default device ID
        blksize: BLOCK_SIZE,          // Default block size
        flags: 0,                     // Default flags
    };
    t.into()
}

pub fn default_file_entry(inode:u64) -> Entry {
    Entry{
        inode,
        generation: 0,
        attr:  default_stat64(inode),
        attr_flags: 0,
        attr_timeout: Duration::from_secs(u64::MAX),
        entry_timeout: Duration::from_secs(u64::MAX),
    } // Return a default Entry instance
}

pub fn default_dic_entry(inode:u64) -> Entry {
    let mut d = default_stat64(inode);
    d.st_mode = 0o0040755;
    Entry{
        inode,
        generation: 0,
        attr:  d,
        attr_flags: 0,
        attr_timeout: Duration::from_secs(u64::MAX),
        entry_timeout: Duration::from_secs(u64::MAX),
    } // Return a default Dictionary Entry instance
}
// pub struct stat64 {
//     pub st_dev: ::dev_t,          // Device ID of the device containing the file
//     pub st_ino: ::ino64_t,        // Inode number of the file
//     pub st_nlink: ::nlink_t,      // Number of hard links to the file
//     pub st_mode: ::mode_t,        // File type and mode (permissions)
//     pub st_uid: ::uid_t,          // User ID of the file's owner
//     pub st_gid: ::gid_t,          // Group ID of the file's owner
//     __pad0: ::c_int,              // Padding for alignment (not used)
//     pub st_rdev: ::dev_t,         // Device ID (if the file is a special file)
//     pub st_size: ::off_t,         // Total size of the file in bytes
//     pub st_blksize: ::blksize_t,  // Block size for filesystem I/O
//     pub st_blocks: ::blkcnt64_t,  // Number of blocks allocated for the file
//     pub st_atime: ::time_t,       // Time of last access
//     pub st_atime_nsec: i64,       // Nanoseconds of last access time
//     pub st_mtime: ::time_t,       // Time of last modification
//     pub st_mtime_nsec: i64,       // Nanoseconds of last modification time
//     pub st_ctime: ::time_t,       // Time of last status change
//     pub st_ctime_nsec: i64,       // Nanoseconds of last status change time
//     __reserved: [i64; 3],         // Reserved for future use (not used)
// }