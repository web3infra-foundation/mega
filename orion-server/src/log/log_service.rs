use std::{path::Path, sync::Arc};

use futures::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;

use crate::log::store::LogStore;

#[derive(Clone, Debug)]
pub struct LogEvent {
    pub task_id: String,
    pub repo_name: String,
    pub build_id: String,
    pub line: String,
    pub is_end: bool,
}

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
        build_id: &str,
    ) -> anyhow::Result<String> {
        let key = self
            .local_log_store
            .get_key(task_id, &Self::last_segment(repo), build_id);

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
        build_id: &str,
        start: usize,
        end: usize,
    ) -> anyhow::Result<String> {
        let key = self
            .local_log_store
            .get_key(task_id, &Self::last_segment(repo), build_id);

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

    pub async fn watch_logs(&self) {
        // Each watcher must have its own receiver
        let mut rx = self.tx.subscribe();

        loop {
            match rx.recv().await {
                Ok(event) => {
                    // First append to local log store
                    let key = self.local_log_store.get_key(
                        &event.task_id,
                        &event.repo_name,
                        &event.build_id,
                    );
                    if let Err(e) = self.local_log_store.append(&key, &event.line).await {
                        tracing::error!(
                            "failed to append log to local store, key={}, error={:?}",
                            key,
                            e
                        );
                    }

                    if event.is_end && self.cloud_upload_enabled {
                        let key = self.cloud_log_store.get_key(
                            &event.task_id,
                            &event.repo_name,
                            &event.build_id,
                        );

                        match self.local_log_store.read(&key).await {
                            Ok(local_content) => {
                                if let Err(e) =
                                    self.cloud_log_store.append(&key, &local_content).await
                                {
                                    tracing::error!(
                                        "failed to append log to cloud store, key={}, error={:?}",
                                        key,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "failed to read local log, key={}, error={:?}",
                                    key,
                                    e
                                );
                            }
                        }
                    }
                }

                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break; // Sender dropped, stop watching
                }

                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!("log receiver lagged, skipped {} messages", skipped);
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::config::{LocalConfig, ObjectStorageBackend, ObjectStorageConfig};
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

        let object_store_wrapper = io_orbit::factory::ObjectStorageFactory::build(
            ObjectStorageBackend::Local,
            &object_storage_config,
        )
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

        let watch_service = log_service.clone();
        let watch_handle = tokio::spawn(async move {
            watch_service.watch_logs().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        log_service.publish(LogEvent {
            task_id: task_id.to_string(),
            repo_name: repo_name.to_string(),
            build_id: build_id.to_string(),
            line: "line 1\n".to_string(),
            is_end: false,
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        log_service.publish(LogEvent {
            task_id: task_id.to_string(),
            repo_name: repo_name.to_string(),
            build_id: build_id.to_string(),
            line: "line 2\n".to_string(),
            is_end: true,
        });

        for _ in 0..20 {
            if local_store.log_exists(&key).await {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert!(local_store.log_exists(&key).await, "local log should exist");

        for _ in 0..20 {
            if cloud_store.log_exists(&key).await {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert!(cloud_store.log_exists(&key).await, "cloud log should exist");

        let local_content = local_store.read(&key).await.unwrap();
        assert!(local_content.contains("line 1"));
        assert!(local_content.contains("line 2"));

        let cloud_content = cloud_store.read(&key).await.unwrap();
        assert!(cloud_content.contains("line 1"));
        assert!(cloud_content.contains("line 2"));

        watch_handle.abort();
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

        let range = log_service
            .read_log_range(task_id, repo_name, build_id, 1, 3)
            .await
            .unwrap();
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
}
