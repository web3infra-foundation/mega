use anyhow::{Result, bail};

use crate::log::store::LogStore;

#[derive(Debug, Default)]
pub struct NoopLogStore;

impl NoopLogStore {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LogStore for NoopLogStore {
    async fn append(&self, _key: &str, _data: &str) -> Result<()> {
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<String> {
        bail!("log storage disabled for key: {}", key)
    }

    async fn delete(&self, _key: &str) -> Result<()> {
        Ok(())
    }

    async fn read_range(&self, key: &str, _start_line: usize, _end_line: usize) -> Result<String> {
        bail!("log storage disabled for key: {}", key)
    }

    async fn log_exists(&self, _key: &str) -> bool {
        false
    }
}
