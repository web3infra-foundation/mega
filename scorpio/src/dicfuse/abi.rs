use std::time::Duration;

use rfuse3::{
    raw::reply::{FileAttr, ReplyEntry},
    FileType, Timestamp,
};

pub fn default_file_entry(inode: u64) -> ReplyEntry {
    ReplyEntry {
        ttl: Duration::new(500, 0),
        attr: FileAttr {
            ino: inode,
            size: 0,
            blocks: 0,
            atime: Timestamp::new(0, 0),
            mtime: Timestamp::new(0, 0),
            ctime: Timestamp::new(0, 0),
            kind: FileType::RegularFile,
            perm: 0o755,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
        },
        generation: 0,
    }
}

pub fn default_dic_entry(inode: u64) -> ReplyEntry {
    ReplyEntry {
        ttl: Duration::new(500, 0),
        attr: FileAttr {
            ino: inode,
            size: 0,
            blocks: 0,
            atime: Timestamp::new(0, 0),
            mtime: Timestamp::new(0, 0),
            ctime: Timestamp::new(0, 0),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
        },
        generation: 0,
    }
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
