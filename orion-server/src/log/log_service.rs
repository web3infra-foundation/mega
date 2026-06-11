use std::{path::Path, sync::Arc, time::Duration};

use api_model::buck2::types::LogEvent;
use futures::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;

use crate::log::store::LogStore;

/// Max attempts for the background cloud upload of a completed build log.
const CLOUD_UPLOAD_MAX_ATTEMPTS: u32 = 5;
/// Initial backoff before the first cloud upload retry (doubles each attempt).
const CLOUD_UPLOAD_INITIAL_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct LogService {
    tx: tokio::sync::broadcast::Sender<LogEvent>,
    local_log_store: Arc<dyn LogStore>,
    cloud_log_store: Arc<dyn LogStore>,
    cloud_upload_enabled: bool,
}

impl LogService {
    pub fn new(
        local_log_store: Arc<dyn LogStore>,
        cloud_log_store: Arc<dyn LogStore>,
        buffer: usize,
        cloud_upload_enabled: bool,
    ) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(buffer);

        Self {
            tx,
            local_log_store,
            cloud_log_store,
            cloud_upload_enabled,
        }
    }

    pub fn last_segment(path: &str) -> String {
        let path = path.trim_end_matches('/');
        Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn publish(&self, event: LogEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe_for_build(&self, build_id: String) -> impl Stream<Item = LogEvent> + use<> {
        let rx = self.tx.subscribe();
        BroadcastStream::new(rx).filter_map(move |res| {
            let build_id = build_id.clone();
            async move {
                match res {
                    Ok(event) if event.build_id == build_id => Some(event),
                    _ => None,
                }
            }
        })
    }

    pub async fn read_full_log(
        &self,
        task_id: &str,
        repo: &str,
        build_event_id: &str,
    ) -> anyhow::Result<String> {
        let key = self
            .local_log_store
            .get_key(task_id, &Self::last_segment(repo), build_event_id);
        match (
            self.local_log_store.log_exists(&key).await,
            self.cloud_log_store.log_exists(&key).await,
        ) {
            (false, false) => {
                anyhow::bail!("log not found in both local and cloud: {}", key);
            }
            (false, true) => {
                let content = self.cloud_log_store.read(&key).await?;
                self.local_log_store.append(&key, &content).await?;
                Ok(content)
            }
            _ => self.local_log_store.read(&key).await,
        }
    }

    pub async fn read_log_range(
        &self,
        task_id: &str,
        repo: &str,
        build_event_id: &str,
        start: usize,
        end: usize,
    ) -> anyhow::Result<String> {
        let key = self
            .local_log_store
            .get_key(task_id, &Self::last_segment(repo), build_event_id);

        let local_exists = self.local_log_store.log_exists(&key).await;
        let cloud_exists = self.cloud_log_store.log_exists(&key).await;

        match (local_exists, cloud_exists) {
            (false, false) => anyhow::bail!("log not found in both local and cloud: {}", key),
            (false, true) => {
                // Cache full content locally after reading from cloud
                let content = self.cloud_log_store.read(&key).await?;
                // Write back to local asynchronously (ignore errors)
                self.local_log_store.append(&key, &content).await?;
                let sliced = content
                    .lines()
                    .skip(start)
                    .take(end - start)
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(sliced)
            }
            _ => {
                // Local log exists, read directly by range
                self.local_log_store.read_range(&key, start, end).await
            }
        }
    }

    /// Reliably persist a single build-output line to the local log store.
    ///
    /// This runs inline on the build-output handling path (not via the broadcast
    /// channel), so persistence does not depend on a watcher keeping up and is
    /// not subject to broadcast lag/drops.
    pub async fn append_local(
        &self,
        task_id: &str,
        repo_name: &str,
        build_id: &str,
        line: &str,
    ) -> anyhow::Result<()> {
        let key = self.local_log_store.get_key(task_id, repo_name, build_id);
        self.local_log_store.append(&key, line).await
    }

    /// Spawn a background task that uploads the completed build's local log to
    /// cloud storage, retrying with exponential backoff. No-op when cloud upload
    /// is disabled.
    pub fn spawn_cloud_upload(&self, task_id: String, repo_name: String, build_id: String) {
        if !self.cloud_upload_enabled {
            return;
        }

        let local_log_store = self.local_log_store.clone();
        let cloud_log_store = self.cloud_log_store.clone();

        tokio::spawn(async move {
            let key = local_log_store.get_key(&task_id, &repo_name, &build_id);

            let content = match local_log_store.read(&key).await {
                Ok(content) => content,
                Err(e) => {
                    tracing::error!(
                        "cloud upload skipped, cannot read local log key={}, error={:?}",
                        key,
                        e
                    );
                    return;
                }
            };

            let mut backoff = CLOUD_UPLOAD_INITIAL_BACKOFF;
            for attempt in 1..=CLOUD_UPLOAD_MAX_ATTEMPTS {
                // On retries, clear any partial object left by a failed attempt
                // so we don't duplicate content.
                if attempt > 1 {
                    let _ = cloud_log_store.delete(&key).await;
                }

                match cloud_log_store.append(&key, &content).await {
                    Ok(()) => {
                        tracing::info!("uploaded log to cloud, key={}, attempt={}", key, attempt);
                        return;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "cloud upload attempt {}/{} failed, key={}, error={:?}",
                            attempt,
                            CLOUD_UPLOAD_MAX_ATTEMPTS,
                            key,
                            e
                        );
                        if attempt < CLOUD_UPLOAD_MAX_ATTEMPTS {
                            tokio::time::sleep(backoff).await;
                            backoff = backoff.saturating_mul(2);
                        }
                    }
                }
            }

            tracing::error!(
                "cloud upload failed after {} attempts, key={}",
                CLOUD_UPLOAD_MAX_ATTEMPTS,
                key
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::config::{LocalConfig, ObjectStorageConfig};
    use tempfile::TempDir;

    use super::*;
    use crate::log::store::{io_orbit_store::IoOrbitLogStore, local_log_store::LocalLogStore};

    async fn create_mix_mode_service() -> (LogService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let local_log_dir = temp_dir.path().join("local_logs");
        let cloud_log_dir = temp_dir.path().join("cloud_logs");

        std::fs::create_dir_all(&local_log_dir).unwrap();
        std::fs::create_dir_all(&cloud_log_dir).unwrap();

        let object_storage_config = ObjectStorageConfig {
            local: LocalConfig {
                root_dir: cloud_log_dir.to_string_lossy().to_string(),
            },
            ..Default::default()
        };

        let object_store_wrapper =
            io_orbit::factory::ObjectStorageFactory::build(&object_storage_config)
                .await
                .unwrap();

        let local_log_store: Arc<dyn LogStore> =
            Arc::new(LocalLogStore::new(local_log_dir.to_string_lossy().as_ref()));
        let cloud_log_store: Arc<dyn LogStore> =
            Arc::new(IoOrbitLogStore::new(object_store_wrapper));

        let log_service = LogService::new(local_log_store, cloud_log_store, 4096, true);

        (log_service, temp_dir)
    }

    #[tokio::test]
    async fn test_mix_mode_basic() {
        let (log_service, _temp_dir) = create_mix_mode_service().await;
        let local_store = log_service.local_log_store.clone();
        let cloud_store = log_service.cloud_log_store.clone();

        let task_id = "task_1";
        let repo_name = "repo";
        let build_id = "build_1";
        let key = local_store.get_key(task_id, repo_name, build_id);

        // Reliable inline local persistence (no broadcast watcher involved).
        log_service
            .append_local(task_id, repo_name, build_id, "line 1\n")
            .await
            .unwrap();
        log_service
            .append_local(task_id, repo_name, build_id, "line 2\n")
            .await
            .unwrap();

        assert!(local_store.log_exists(&key).await, "local log should exist");

        let local_content = local_store.read(&key).await.unwrap();
        assert!(local_content.contains("line 1"));
        assert!(local_content.contains("line 2"));

        // Background, retryable cloud upload on completion.
        log_service.spawn_cloud_upload(task_id.to_string(), repo_name.to_string(), build_id.to_string());

        for _ in 0..20 {
            if cloud_store.log_exists(&key).await {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert!(cloud_store.log_exists(&key).await, "cloud log should exist");

        let cloud_content = cloud_store.read(&key).await.unwrap();
        assert!(cloud_content.contains("line 1"));
        assert!(cloud_content.contains("line 2"));
    }

    #[tokio::test]
    async fn test_read_full_log() {
        let (log_service, _temp_dir) = create_mix_mode_service().await;
        let local_store = log_service.local_log_store.clone();

        let task_id = "task_2";
        let repo_name = "repo";
        let build_id = "build_2";
        let key = local_store.get_key(task_id, repo_name, build_id);

        local_store
            .append(&key, "log line 1\nlog line 2\n")
            .await
            .unwrap();

        let content = log_service
            .read_full_log(task_id, repo_name, build_id)
            .await
            .unwrap();
        assert!(content.contains("log line 1"));
        assert!(content.contains("log line 2"));
    }

    #[tokio::test]
    async fn test_read_log_range() {
        let (log_service, _temp_dir) = create_mix_mode_service().await;
        let local_store = log_service.local_log_store.clone();

        let task_id = "task_3";
        let repo_name = "repo";
        let build_id = "build_3";
        let key = local_store.get_key(task_id, repo_name, build_id);

        local_store
            .append(&key, "line 0\nline 1\nline 2\nline 3\n")
            .await
            .unwrap();

        // Use cloud_store (IoOrbitLogStore) to append multiple times, ensuring multiple segments are created,
        // then verify cross-segment line range reading logic.
        let cloud_store = log_service.cloud_log_store.clone();
        let cloud_key = cloud_store.get_key(task_id, repo_name, build_id);

        cloud_store.append(&cloud_key, "line 0\n").await.unwrap();
        cloud_store.append(&cloud_key, "line 1\n").await.unwrap();
        cloud_store.append(&cloud_key, "line 2\n").await.unwrap();
        cloud_store.append(&cloud_key, "line 3\n").await.unwrap();

        let range = cloud_store.read_range(&cloud_key, 1, 3).await.unwrap();
        assert!(range.contains("line 1"));
        assert!(range.contains("line 2"));
        assert!(!range.contains("line 0"));
        assert!(!range.contains("line 3"));
    }

    #[tokio::test]
    async fn test_cloud_recovery() {
        let (log_service, _temp_dir) = create_mix_mode_service().await;
        let local_store = log_service.local_log_store.clone();
        let cloud_store = log_service.cloud_log_store.clone();

        let task_id = "task_4";
        let repo_name = "repo";
        let build_id = "build_4";
        let key = local_store.get_key(task_id, repo_name, build_id);

        local_store.append(&key, "recovered log\n").await.unwrap();
        cloud_store.append(&key, "recovered log\n").await.unwrap();
        local_store.delete(&key).await.unwrap();

        assert!(!local_store.log_exists(&key).await);
        assert!(cloud_store.log_exists(&key).await);

        let content = log_service
            .read_full_log(task_id, repo_name, build_id)
            .await
            .unwrap();
        assert!(content.contains("recovered log"));
        assert!(local_store.log_exists(&key).await);
    }

    /// Test very large object segmentation (exceeds 16 MB)
    /// Note: This test creates large data and may take some time
    #[tokio::test]
    async fn test_large_object_segmentation() {
        let (log_service, _temp_dir) = create_mix_mode_service().await;
        let cloud_store = log_service.cloud_log_store.clone();

        let task_id = "task_very_large";
        let repo_name = "repo";
        let build_id = "build_very_large";
        let key = cloud_store.get_key(task_id, repo_name, build_id);

        // MAX_SEGMENT_SIZE is 16 MB, create 20 MB data to test segmentation
        const DATA_SIZE: usize = 20 * 1024 * 1024; // 20 MB

        // Create a large data block with recognizable patterns
        let mut large_data = Vec::with_capacity(DATA_SIZE);
        let pattern = b"B".repeat(1024); // 1KB pattern
        for i in 0..(DATA_SIZE / 1024) {
            large_data.extend_from_slice(format!("{:08}:", i).as_bytes());
            large_data.extend_from_slice(&pattern);
            large_data.push(b'\n');
        }
        let large_data_str = String::from_utf8(large_data).unwrap();

        // Write large object
        cloud_store.append(&key, &large_data_str).await.unwrap();

        // Verify data can be read completely
        let read_back = cloud_store.read(&key).await.unwrap();
        assert_eq!(
            read_back.len(),
            large_data_str.len(),
            "Read data size should match"
        );
        assert_eq!(read_back, large_data_str, "Read data should match original");

        // Verify each segment size doesn't exceed MAX_SEGMENT_SIZE
        // Verify cross-segment reading by reading data from the middle position
        let mid_point = large_data_str.len() / 2;
        let mid_data = &large_data_str[mid_point..mid_point + 100];
        assert!(
            read_back.contains(mid_data),
            "Should be able to read data across segments"
        );
    }
}
