use libc::{_SC_PAGESIZE,sysconf};
#[repr(C)]
pub struct MemoryInfoC {
    pub free_memory: u64,
    pub active_memory: u64,
    pub inactive_memory: u64,
    pub wired_memory: u64,
    pub total_memory: u64,
}

extern "C" {
    pub fn get_memory_info(info: &mut MemoryInfoC) -> i32;
}

pub struct MemoryInfo {
    pub page_size: usize,
}
impl MemoryInfo {
    pub fn new() -> Option<Self> {
        let page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };
        if page_size > 0 {
            Some(MemoryInfo {page_size})
        } else {
            None
        }
    }
    pub(crate) fn get_free_memory(&self) -> Option<MemoryInfoC> {
        let mut mem_info = MemoryInfoC {
            free_memory: 0,
            active_memory: 0,
            inactive_memory: 0,
            wired_memory: 0,
            total_memory: 0,
        };

        let result = unsafe { get_memory_info(&mut mem_info) };

        if result == 0 {
            Some(mem_info)
        } else {
            panic!("Failed to fetch memory information");
            None
        }
    }
}