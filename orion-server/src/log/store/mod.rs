use anyhow::Result;

pub mod local_log_store;
pub mod s3_log_store;

/// Trait representing a generic log storage backend.
///
/// Implementors provide methods to append, read, delete, and read ranges from log files or objects.
///
/// # Note
/// * For `LocalLogStore`, the `key` parameter corresponds directly to the file path.
/// * For cloud/object storage (S3, Other OSS), the `key` parameter corresponds to the object key in the bucket.
#[async_trait::async_trait]
pub trait LogStore: Send + Sync {
    /// Appends data to the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - The log identifier. For local storage, this is the file path; for cloud storage, this is the object key.
    /// * `data` - The string data to append to the log.
    ///
    /// # Returns
    /// * `Ok(())` - If the append operation succeeds.
    /// * `Err` - Returns an error if the append fails, e.g., file system error, permission denied, or cloud operation failure.
    async fn append(&self, key: &str, data: &str) -> anyhow::Result<()>;

    /// Reads the full contents of the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - The log identifier. For local storage, this is the file path; for cloud storage, this is the object key.
    ///
    /// # Returns
    /// * `Ok(String)` - The complete log content as a string.
    /// * `Err` - Returns an error if reading fails, e.g., if the file does not exist or cloud access fails.
    async fn read(&self, key: &str) -> Result<String>;

    /// Deletes the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - The log identifier. For local storage, this is the file path; for cloud storage, this is the object key.
    ///
    /// # Returns
    /// * `Ok(())` - If the deletion succeeds.
    /// * `Err` - Returns an error if the deletion fails, e.g., file not found, permission denied, or cloud operation failure.
    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> anyhow::Result<()>;

    /// Reads a range of lines from the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - The log identifier. For local storage, this is the file path; for cloud storage, this is the object key.
    /// * `start_line` - Starting line index (inclusive)
    /// * `end_line` - Ending line index (exclusive)
    ///
    /// # Returns
    /// * `Ok(String)` - The log content for the specified line range.
    /// * `Err` - Returns an error if reading fails, e.g., file not found, permission denied, or cloud operation failure.
    async fn read_range(
        &self,
        key: &str,
        start_line: usize,
        end_line: usize,
    ) -> anyhow::Result<String>;

    /// Checks whether a log identified by `key` exists.
    ///
    /// # Arguments
    /// * `key` - The log identifier. For local storage, this is the file path; for cloud storage, this is the object key.
    ///
    /// # Returns
    /// * `true` if the log exists.
    /// * `false` if the log does not exist or cannot be accessed.
    async fn log_exists(&self, key: &str) -> bool;

    /// Generate the storage key for a given task/repo/build.
    ///
    /// # Arguments
    /// * `task_id` - Identifier for the task
    /// * `repo_name` - Name of the repository
    /// * `build_id` - Identifier for the build
    ///
    /// # Returns
    /// * `String` representing the log key (file path for local storage, object key for cloud storage)
    fn get_key(&self, task_id: &str, repo_name: &str, build_id: &str) -> String {
        format!("{}/{}/{}.log", task_id, repo_name, build_id)
    }
}
