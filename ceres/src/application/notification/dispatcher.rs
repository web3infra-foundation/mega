use std::sync::Arc;

use async_trait::async_trait;
use common::errors::MegaError;
use jupiter::storage::NotificationStorage;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

/// Sends notification email payloads produced by the notification application layer.
#[async_trait]
pub trait EmailMailer: Send + Sync {
    async fn send_html(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: Option<&str>,
    ) -> Result<(), MegaError>;
}

pub struct EmailDispatcher {
    stg: NotificationStorage,
    mailer: Arc<dyn EmailMailer>,
}

impl EmailDispatcher {
    pub fn new(stg: NotificationStorage, mailer: Arc<dyn EmailMailer>) -> Self {
        Self { stg, mailer }
    }

    pub async fn run(self, shutdown: tokio_util::sync::CancellationToken) {
        let mut tick = interval(Duration::from_secs(2));

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!("email dispatcher shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.tick_once().await {
                        warn!("email dispatcher tick error: {e}");
                    }
                }
            }
        }
    }

    pub async fn tick_once(&self) -> Result<(), jupiter::sea_orm::DbErr> {
        let jobs = self.stg.fetch_pending_jobs(50).await?;
        for job in jobs {
            if job.to_email.trim().is_empty() {
                let _ = self
                    .stg
                    .mark_job_skipped(job.id, "missing recipient email")
                    .await;
                continue;
            }
            if !self.stg.try_claim_job(job.id).await? {
                continue;
            }
            let send_res = self
                .mailer
                .send_html(
                    &job.to_email,
                    &job.subject,
                    &job.body_html,
                    job.body_text.as_deref(),
                )
                .await;

            match send_res {
                Ok(_) => {
                    let _ = self.stg.mark_job_sent(job.id).await;
                }
                Err(e) => {
                    let _ = self
                        .stg
                        .mark_job_failed_with_retry(job.id, &format!("{e:?}"))
                        .await;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use common::errors::MegaError;
    use jupiter::tests::test_db_connection;
    use jupiter_migrate::apply_migrations;
    use tempfile::TempDir;

    use super::*;
    use crate::application::notification::ensure_cl_comment_event_type;

    struct NoopMailer;

    #[async_trait]
    impl EmailMailer for NoopMailer {
        async fn send_html(
            &self,
            _to: &str,
            _subject: &str,
            _html: &str,
            _text: Option<&str>,
        ) -> Result<(), MegaError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_dispatcher_sends_pending_jobs() {
        let dir = TempDir::new().unwrap();
        let db = test_db_connection(dir.path()).await;
        apply_migrations(&db, true).await.unwrap();

        let stg = NotificationStorage::new(Arc::new(db));
        ensure_cl_comment_event_type(&stg).await.unwrap();
        stg.enqueue_email_job(
            "alice",
            "alice@example.com",
            "cl.comment.created",
            "Subject",
            "<p>Body</p>",
            Some("Body"),
        )
        .await
        .unwrap();

        let pending_before = stg.fetch_pending_jobs(10).await.unwrap();
        assert_eq!(pending_before.len(), 1);

        let dispatcher = EmailDispatcher::new(stg.clone(), Arc::new(NoopMailer));
        dispatcher.tick_once().await.unwrap();

        let jobs = stg.fetch_pending_jobs(10).await.unwrap();
        assert!(jobs.is_empty(), "pending queue should be empty after send");
    }
}
