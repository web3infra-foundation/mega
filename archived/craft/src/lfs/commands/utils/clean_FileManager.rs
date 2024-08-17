use std::{fs, io};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use libc::{_SC_PAGESIZE, c_long, sysconf};
use memmap2::MmapOptions;
use sha2::{Digest, Sha256};
use crate::lfs::commands::utils::os_vm_utils::{MemoryInfo, MemoryInfoC};
use rayon::iter::ParallelIterator;


pub struct FileManager {
    file_info:FileInfo,
    max_memory_usage: u64,
    memory_info: MemoryInfo,
}
#[derive(Clone)]
struct FileInfo {
    file_path : String,
    file_size : u64
}

impl FileManager {
    pub  fn new(file_path : PathBuf) -> Option<Self> {
        let memory_info = MemoryInfo::new()?;
        if let Some(memory_info_c) = memory_info.get_free_memory() {
            let max_memory_usage = memory_info_c.free_memory + memory_info_c.inactive_memory;
            let file_info = Self::create_file_info(&file_path);
            return Some(FileManager {
                file_info,
                max_memory_usage,
                memory_info,
            })
        } else {
            None
        }

    }
    fn create_file_info<P:AsRef<Path>>(path:P) -> FileInfo {
        let file_size = Self::get_file_size(&path).expect("!!");
        FileInfo {
            file_path: path.as_ref().to_string_lossy().into_owned(),
            file_size
        }
    }
    fn get_file_size<P:AsRef<Path>>(file_path:P) -> Option<u64> {
        fs::metadata(file_path).ok()?.len().into()
    }
    pub(crate) fn run(self) -> std::io::Result<(String,u64)> {
        let result = self.process_files()?;
        Ok(result)
    }
    fn process_files(self) -> std::io::Result<(String,u64)> {
        self.compute_sha256()
    }

    fn check_memory_usage(&self) -> Option<u64> {
        Some(self.memory_info.get_free_memory().unwrap().free_memory + self.memory_info.get_free_memory().unwrap().inactive_memory)
    }

    fn vm_page_size() -> c_long {
        unsafe { sysconf(_SC_PAGESIZE) }
    }
    fn compute_sha256(&self) -> io::Result<(String,u64)> {
        let file = File::open(&self.file_info.file_path)?;
        let file_size = &self.file_info.file_size;

        let mut hasher = Sha256::new();

        if file_size <= &(((&self.memory_info.page_size) * 1024) as u64) {
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            hasher.update(&buffer);
        } else {
            if file_size <= &self.max_memory_usage {
                let mmap = unsafe { MmapOptions::new().map(&file)? };
                hasher.update(&mmap);
            } else {
                let mut reader = BufReader::with_capacity(self.max_memory_usage as usize, file);
                let mut buffer = vec![0; self.max_memory_usage as usize];
                loop {
                    let bytes_read = reader.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
            }
        }
        Ok((format!("{:x}", hasher.finalize()), *file_size))
    }
}