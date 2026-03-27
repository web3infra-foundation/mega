use std::sync::Arc;

use dashmap::DashMap;
use sea_orm::DatabaseConnection;
use tokio::sync::watch;

use crate::{
    log::log_service::LogService, repository::target_build_status_repo::TargetBuildStatusRepo,
    scheduler::TaskScheduler, service::target_status_cache_service::TargetStatusCache,
};

async fn target_status_cache_flush_loop(
    cache: TargetStatusCache,
    conn: DatabaseConnection,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let models = cache.flush_all().await;
                if models.is_empty() {
                    continue;
                }
                if let Err(e) = TargetBuildStatusRepo::upsert_batch(&conn, models).await {
                    tracing::error!("Auto flush failed: {:?}", e);
                }
            }
            _ = shutdown.changed() => {
                tracing::info!("TargetStatusCache auto flush shutting down");
                let models = cache.flush_all().await;
                if !models.is_empty() {
                    let _ = TargetBuildStatusRepo::upsert_batch(&conn, models).await;
                }
                break;
            }
        }
    }
}

/// Shared application state containing worker connections, database, and active builds.
#[derive(Clone)]
pub struct AppState {
    pub scheduler: TaskScheduler,
    pub conn: DatabaseConnection,
    pub log_service: LogService,
    pub target_status_cache: TargetStatusCache,
    shutdown_tx: watch::Sender<bool>,
}

impl AppState {
    pub fn new(
        conn: DatabaseConnection,
        queue_config: Option<crate::scheduler::TaskQueueConfig>,
        log_service: LogService,
    ) -> Self {
        let workers = Arc::new(DashMap::new());
        let active_builds = Arc::new(DashMap::new());
        let scheduler = TaskScheduler::new(conn.clone(), workers, active_builds, queue_config);
        let target_status_cache = TargetStatusCache::new();
        let (shutdown_tx, _) = watch::channel(false);

        Self {
            scheduler,
            conn,
            log_service,
            target_status_cache,
            shutdown_tx,
        }
    }

    pub fn start_background_tasks(&self) {
        let conn = self.conn.clone();
        let cache = self.target_status_cache.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            target_status_cache_flush_loop(cache, conn, shutdown_rx).await;
        });
    }

    pub async fn start_queue_manager(self) {
        self.scheduler.start_queue_manager().await;
    }
}
