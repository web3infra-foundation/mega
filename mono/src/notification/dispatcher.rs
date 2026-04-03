use std::sync::Arc;

use jupiter::storage::NotificationStorage;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

use crate::email::Mailer;

pub struct EmailDispatcher {
    stg: NotificationStorage,
    mailer: Arc<dyn Mailer>,
}

impl EmailDispatcher {
    pub fn new(stg: NotificationStorage, mailer: Arc<dyn Mailer>) -> Self {
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

    async fn tick_once(&self) -> Result<(), jupiter::sea_orm::DbErr> {
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
    use callisto::{email_jobs, notification_event_types};
    use jupiter::{
        migration::apply_migrations,
        sea_orm::{ActiveModelTrait, EntityTrait, Set},
        tests::test_db_connection,
    };
    use tempfile::TempDir;

    use super::*;
    use crate::email::NoopMailer;

    #[tokio::test]
    async fn test_dispatcher_sends_pending_jobs() {
        let dir = TempDir::new().unwrap();
        let db = test_db_connection(dir.path()).await;
        apply_migrations(&db, true).await.unwrap();

        let stg = NotificationStorage::new(Arc::new(db.clone()));
        let now = chrono::Utc::now().naive_utc();

        // ensure event type exists
        notification_event_types::ActiveModel {
            code: Set("cl.comment.created".into()),
            category: Set("cl".into()),
            description: Set("New comment".into()),
            system_required: Set(false),
            default_enabled: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // enqueue a job
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

        let dispatcher = EmailDispatcher::new(stg.clone(), Arc::new(NoopMailer));
        dispatcher.tick_once().await.unwrap();

        let jobs = stg.fetch_pending_jobs(10).await.unwrap();
        assert!(jobs.is_empty(), "pending queue should be empty after send");

        let sent = email_jobs::Entity::find().all(&db).await.unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].status, "sent");
    }
}
