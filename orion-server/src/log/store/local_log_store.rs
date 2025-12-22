use std::io::{BufRead, Write};

use anyhow::Result;

use crate::log::store::LogStore;
pub struct LocalLogStore {
    base_path: String,
}

impl LocalLogStore {
    pub fn new(base_path: &str) -> Self {
        LocalLogStore {
            base_path: base_path.to_string(),
        }
    }

    /// Build the full file system path from the given log key.
    ///
    /// For `LocalLogStore`, the `key` represents a relative file path
    /// (for example: `task_id/repo_name/build_id`).
    /// This method prefixes it with the base `log_path` to produce
    /// an absolute or root-relative file system path.
    ///
    /// # Arguments
    ///
    /// * `key` - Log identifier used as a relative path
    ///
    /// # Returns
    ///
    /// * Full file path as a `String`
    fn get_file_path(&self, key: &str) -> String {
        format!("{}/{}", self.base_path, key)
    }

    /// Open an existing log file or create it if it does not exist for a specific path.
    ///
    /// This function ensures that all parent directories exist, and then opens the file
    /// in append mode. If the file does not exist, it will be created.
    ///
    /// # Arguments
    ///
    /// * `log_path` - Full path to the log file
    ///
    /// # Returns
    ///
    /// * `Ok(std::fs::File)` - File handle opened in append mode
    /// * `Err(std::io::Error)` - Any IO error that occurs during directory creation or file opening
    fn open_log_file(&self, log_path: &str) -> Result<std::fs::File, std::io::Error> {
        let path = std::path::Path::new(log_path);

        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open the file in append mode (creates it if it doesn't exist)
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    }
}

#[async_trait::async_trait]
impl LogStore for LocalLogStore {
    async fn append(&self, key: &str, data: &str) -> Result<()> {
        let path = self.get_file_path(key);
        // Open the log file (creates if not exist)
        let mut file = self.open_log_file(&path)?;

        // Write data and flush
        if data.ends_with('\n') {
            write!(file, "{}", data)?;
        } else {
            writeln!(file, "{}", data)?;
        }
        file.flush()?;

        Ok(())
    }

    async fn read(&self, key: &str) -> Result<String> {
        let path = self.get_file_path(key);
        let content = std::fs::read_to_string(&path)?;
        Ok(content)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.get_file_path(key);
        if std::fs::metadata(&path).is_ok() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    async fn read_range(&self, key: &str, start_line: usize, end_line: usize) -> Result<String> {
        let path = self.get_file_path(key);
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);

        let lines: Vec<String> = reader
            .lines()
            .skip(start_line)
            .take(end_line.saturating_sub(start_line))
            .collect::<Result<_, _>>()?;

        Ok(lines.join("\n"))
    }

    async fn log_exists(&self, key: &str) -> bool {
        let path = self.get_file_path(key);
        match std::fs::metadata(&path) {
            Ok(meta) => meta.is_file(),
            Err(_) => false,
        }
    }
}
