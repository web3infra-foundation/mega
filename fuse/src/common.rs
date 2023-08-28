use std::sync::atomic::AtomicU32;

static GID: AtomicU32 = AtomicU32::new(1000);
static UID: AtomicU32 = AtomicU32::new(1000);
pub const BLOCK_SIZE: u32 = 4096;
pub const DEFAULT_HARD_LINKS:u32=1;
pub const RDEV:u32=0;
pub const FLAGS:u32=0;
pub const MAX_NAME_LENGTH:u32=255;
pub const DEFAULT_PERMISSIONS:u16=600;
pub const FMODE_EXEC: i32 = 0x20;

pub fn init_gu_id(gid: u32, uid: u32) {
    GID.store(gid, std::sync::atomic::Ordering::SeqCst);
    UID.store(uid, std::sync::atomic::Ordering::SeqCst);
}

pub fn gid() -> u32 {
    GID.load(std::sync::atomic::Ordering::Acquire)
}

pub fn uid() -> u32 {
    UID.load(std::sync::atomic::Ordering::Acquire)
}

