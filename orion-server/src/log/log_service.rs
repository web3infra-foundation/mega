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
