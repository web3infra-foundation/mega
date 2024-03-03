use std::collections::VecDeque;
use std::{fs, io};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use libc::{_SC_PAGESIZE, c_long, sysconf};
use memmap2::MmapOptions;
use rayon::iter::IntoParallelRefIterator;
use sha2::{Digest, Sha256};
use crate::lfs::commands::utils::os_vm_utils::{MemoryInfo, MemoryInfoC};
use rayon::iter::ParallelIterator;

pub struct FileManager {
    files_queue: VecDeque<FileInfo>,
    max_memory_usage: u64,
    memory_info: MemoryInfo,
}
#[derive(Clone)]
struct FileInfo {
    file_path : String,
    file_size : u64
}

impl FileManager {
    pub  fn new(file_paths: Vec<String>) -> Option<Self> {
        let memory_info = MemoryInfo::new()?;
        if let Some(memory_info_c) = memory_info.get_free_memory() {
            let max_memory_usage = memory_info_c.free_memory + memory_info_c.inactive_memory;
            let files_queue :VecDeque<FileInfo> = file_paths
                .into_iter()
                .map(|path|Self::create_file_info(&path))
                .collect();
            return Some(FileManager {
                files_queue,
                max_memory_usage,
                memory_info,
            })
        } else {
            None
        }

    }
    fn create_file_info<P:AsRef<Path>>(path:P) -> FileInfo {
        let file_size = Self::get_file_size(&path).expect("Error");
        FileInfo {
            file_path: path.as_ref().to_string_lossy().into_owned(),
            file_size
        }
    }
    fn get_file_size<P:AsRef<Path>>(file_path:P) -> Option<u64> {
        fs::metadata(file_path).ok()?.len().into()
    }
    pub(crate) fn run(&mut self) {
        self.process_files()
    }

    fn find_and_remove_combination(&mut self, max_memory_usage:u64) -> (Vec<FileInfo>, Vec<FileInfo>) {
        let mut combination = Vec::new();
        let mut oversized_files = Vec::new();
        let mut total_size = 0;

        while let Some(file_info) = self.files_queue.front().cloned() {
            if file_info.file_size > max_memory_usage {
                oversized_files.push(self.files_queue.pop_front().unwrap());
                continue;
            }
            if total_size + file_info.file_size > max_memory_usage {
                break;
            }
            combination.push(self.files_queue.pop_front().unwrap());
            total_size += file_info.file_size;
        }

        (combination, oversized_files)
    }

    fn get_max_memory_usage(&self) -> u64 {
        if let Some(memory_info_c) = self.memory_info.get_free_memory() {
            let max_memory_usage = memory_info_c.free_memory + memory_info_c.inactive_memory;
            return max_memory_usage
        } else {
            panic!("Error!")
        }
    }
    fn process_files(&mut self) {
        let mut max_memory_usage = self.get_max_memory_usage();
        while !self.files_queue.is_empty() {
            let (combination, oversized_files) = &self.find_and_remove_combination( max_memory_usage);
            if !combination.is_empty() {
                let file_paths: Vec<String> = combination.iter().map(|file| file.file_path.clone()).collect();
                let _sha256_results = &self.parallel_sha256(&file_paths);
                for result in _sha256_results {
                    match result {
                        Ok(sha256) => println!("{}", sha256),
                        Err(e) => panic!("Error: {}", e),
                    }
                }
            }
            for file_info in oversized_files {
                let file_path = Path::new(&file_info.file_path);
                let _sha256_result = &self.compute_sha256(file_path);
                match _sha256_result {
                    Ok(sha256) => println!("{}:",  sha256),
                    Err(e) => panic!("Error: {}", e),
                }
            }
            
            max_memory_usage = self.get_max_memory_usage();
        }
    }

    fn check_memory_usage(&self) -> Option<u64> {
        Some(self.memory_info.get_free_memory().unwrap().free_memory + self.memory_info.get_free_memory().unwrap().inactive_memory)
    }

    fn vm_page_size() -> c_long {
        unsafe { sysconf(_SC_PAGESIZE) }
    }
    fn compute_sha256(&self,file_path: &Path) -> io::Result<String> {
        let file = File::open(file_path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        let mut hasher = Sha256::new();

        if file_size <= ((&self.memory_info.page_size) * 1024) as u64 {
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            hasher.update(&buffer);
        } else {
            if file_size <= self.max_memory_usage {
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

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn parallel_sha256(&self,file_paths: &[String]) -> Vec<io::Result<String>> {
        file_paths.par_iter()
            .map(|file_path| {
                let path = Path::new(file_path);
                self.compute_sha256(path)
            })
            .collect()
    }
    fn wait_for_memory(&self) {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}