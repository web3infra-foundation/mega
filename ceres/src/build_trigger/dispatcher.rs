use std::sync::Arc;

use api_model::buck2::{api::TaskBuildRequest, status::Status, types::ProjectRelativePath};
use bellatrix::Bellatrix;
use common::errors::MegaError;
use jupiter::storage::Storage;

use crate::build_trigger::{BuildTrigger, BuildTriggerPayload};

/// Handles dispatching build triggers to the build execution layer (Bellatrix/Orion).
pub struct BuildDispatcher {
    storage: Storage,
    bellatrix: Arc<Bellatrix>,
}

fn payload_to_task_request(payload: &BuildTriggerPayload) -> Result<TaskBuildRequest, MegaError> {
    let (cl_link, repo, builds_json, cl_id) = match payload {
        BuildTriggerPayload::GitPush(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::Manual(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::Retry(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::Webhook(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::Schedule(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::WebEdit(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        BuildTriggerPayload::BuckFileUpload(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
    };

    let changes: Vec<Status<ProjectRelativePath>> = serde_json::from_value(builds_json.clone())
        .map_err(|e| {
            tracing::error!("Failed to deserialize builds from payload: {}", e);
            MegaError::Other(format!("Failed to deserialize builds from payload: {}", e))
        })?;

    Ok(TaskBuildRequest {
        repo: repo.to_string(),
        cl_link: cl_link.to_string(),
        cl_id: cl_id.unwrap_or(0),
        changes,
        targets: None,
    })
}

impl BuildDispatcher {
    pub fn new(storage: Storage, bellatrix: Arc<Bellatrix>) -> Self {
        Self { storage, bellatrix }
    }

    pub async fn dispatch(&self, trigger: BuildTrigger) -> Result<i64, MegaError> {
        let trigger_payload = serde_json::to_value(&trigger.payload).map_err(|e| {
            tracing::error!("Failed to serialize payload: {}", e);
            MegaError::Other(format!("Failed to serialize payload: {}", e))
        })?;

        // Determine task_id based on whether build system is enabled
        let task_id: Option<uuid::Uuid> = if self.bellatrix.enable_build() {
            let req = payload_to_task_request(&trigger.payload)?;

            let task_id_str = self.bellatrix.on_post_receive(req).await.map_err(|e| {
                tracing::error!("Failed to dispatch build to Bellatrix: {}", e);
                MegaError::Other(format!("Failed to dispatch build to Bellatrix: {}", e))
            })?;

            let task_uuid = uuid::Uuid::parse_str(&task_id_str).map_err(|e| {
                tracing::error!("Invalid task_id format '{}': {}", task_id_str, e);
                MegaError::Other(format!("Invalid task_id format '{}': {}", task_id_str, e))
            })?;

            Some(task_uuid)
        } else {
            tracing::info!("BuildDispatcher: Build system disabled, skipping Orion call");
            None
        };

        // Insert trigger record with task_id (complete record in one operation)
        let db_record = self
            .storage
            .build_trigger_storage()
            .insert(
                trigger.trigger_type.to_string(),
                trigger.trigger_source.to_string(),
                trigger_payload,
                task_id,
            )
            .await?;

        tracing::info!(
            "BuildDispatcher: Trigger persisted (ID: {}, Task ID: {:?})",
            db_record.id,
            task_id
        );

        Ok(db_record.id)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use api_model::buck2::ws::WSMessage;
    use axum::{Json, Router, extract::State, routing::post};
    use chrono::Utc;
    use common::config::BuildConfig;
    use tempfile::tempdir;
    use tokio::{net::TcpListener, sync::mpsc};

    use super::*;
    use crate::build_trigger::{BuildTriggerType, TriggerSource, WebEditPayload};

    #[derive(Clone)]
    struct MockOrionState {
        worker_tx: mpsc::UnboundedSender<WSMessage>,
        task_id: String,
    }

    async fn mock_task_handler(
        State(state): State<MockOrionState>,
        Json(req): Json<TaskBuildRequest>,
    ) -> Json<serde_json::Value> {
        let _ = state.worker_tx.send(WSMessage::TaskBuild {
            build_id: state.task_id.clone(),
            repo: req.repo,
            cl_link: req.cl_link,
            changes: req.changes,
        });
        Json(serde_json::json!({ "task_id": state.task_id }))
    }

    async fn spawn_mock_orion(
        worker_tx: mpsc::UnboundedSender<WSMessage>,
        task_id: String,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/v2/task", post(mock_task_handler))
            .with_state(MockOrionState { worker_tx, task_id });
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock orion");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock orion server exits cleanly");
        });

        (format!("http://localhost:{}", addr.port()), handle)
    }

    fn web_edit_trigger(
        repo: &str,
        cl_link: &str,
        changes: Vec<Status<ProjectRelativePath>>,
    ) -> BuildTrigger {
        BuildTrigger {
            trigger_type: BuildTriggerType::WebEdit,
            trigger_source: TriggerSource::User,
            trigger_time: Utc::now(),
            payload: BuildTriggerPayload::WebEdit(WebEditPayload {
                repo: repo.to_string(),
                from_hash: "1".repeat(40),
                commit_hash: "2".repeat(40),
                cl_link: cl_link.to_string(),
                cl_id: Some(101),
                builds: serde_json::to_value(&changes).expect("serialize changes"),
                triggered_by: Some("jackie".to_string()),
            }),
        }
    }

    async fn run_web_edit_chain_case(
        repo: &str,
        cl_link: &str,
        changes: Vec<Status<ProjectRelativePath>>,
    ) {
        let temp_dir = tempdir().expect("create temp dir");
        let storage = jupiter::tests::test_storage(temp_dir.path()).await;
        let (worker_tx, mut worker_rx) = mpsc::unbounded_channel();
        let expected_task_id = uuid::Uuid::now_v7().to_string();
        let (orion_base, mock_orion_handle) =
            spawn_mock_orion(worker_tx, expected_task_id.clone()).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let bellatrix = Arc::new(Bellatrix::new(BuildConfig {
            enable_build: true,
            orion_server: orion_base,
            ..Default::default()
        }));
        let dispatcher = BuildDispatcher::new(storage.clone(), bellatrix);
        let trigger = web_edit_trigger(repo, cl_link, changes.clone());

        let trigger_id = dispatcher
            .dispatch(trigger)
            .await
            .expect("dispatch build trigger");

        let record = storage
            .build_trigger_storage()
            .get_by_id(trigger_id)
            .await
            .expect("read trigger from db")
            .expect("trigger exists");
        assert_eq!(
            record.task_id.map(|id| id.to_string()),
            Some(expected_task_id.clone())
        );

        let worker_msg = tokio::time::timeout(Duration::from_secs(3), worker_rx.recv())
            .await
            .expect("worker message timeout")
            .expect("worker channel closed unexpectedly");
        match worker_msg {
            WSMessage::TaskBuild {
                repo: actual_repo,
                cl_link: actual_cl_link,
                changes: actual_changes,
                build_id,
            } => {
                assert_eq!(build_id, expected_task_id);
                assert_eq!(actual_repo, repo);
                assert_eq!(actual_cl_link, cl_link);
                assert_eq!(actual_changes, changes);
            }
            other => panic!("unexpected worker message: {:?}", other),
        }

        mock_orion_handle.abort();
    }

    #[test]
    fn test_payload_to_task_request_parses_builds_json() {
        let payload = BuildTriggerPayload::WebEdit(WebEditPayload {
            repo: "/project/buck2_test".to_string(),
            from_hash: "1".repeat(40),
            commit_hash: "2".repeat(40),
            cl_link: "HVKM7CXI".to_string(),
            cl_id: Some(88),
            builds: serde_json::to_value(vec![Status::Modified(ProjectRelativePath::new(
                "src/main.rs",
            ))])
            .expect("serialize builds"),
            triggered_by: Some("jackie".to_string()),
        });

        let req = payload_to_task_request(&payload).expect("build task request");
        assert_eq!(req.repo, "/project/buck2_test");
        assert_eq!(req.cl_link, "HVKM7CXI");
        assert_eq!(req.cl_id, 88);
        assert_eq!(
            req.changes,
            vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))]
        );
    }

    #[tokio::test]
    async fn test_dispatch_skips_orion_when_build_disabled_and_persists_trigger() {
        let temp_dir = tempdir().expect("create temp dir");
        let storage = jupiter::tests::test_storage(temp_dir.path()).await;
        let bellatrix = Arc::new(Bellatrix::new(BuildConfig {
            enable_build: false,
            orion_server: "http://127.0.0.1:0".to_string(),
            ..Default::default()
        }));
        let dispatcher = BuildDispatcher::new(storage.clone(), bellatrix);

        let trigger = web_edit_trigger(
            "/project/buck2_test",
            "DISABLEDBUILD",
            vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))],
        );
        let trigger_id = dispatcher
            .dispatch(trigger)
            .await
            .expect("dispatch should still persist");
        let record = storage
            .build_trigger_storage()
            .get_by_id(trigger_id)
            .await
            .expect("read trigger")
            .expect("trigger exists");

        assert!(record.task_id.is_none());
    }

    #[tokio::test]
    async fn test_web_edit_chain_save_file_edit_to_worker_task() {
        run_web_edit_chain_case(
            "/project/buck2_test",
            "HVKM7CXI",
            vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))],
        )
        .await;
    }

    #[tokio::test]
    async fn test_web_edit_chain_create_monorepo_entry_to_worker_task() {
        run_web_edit_chain_case(
            "/project/buck2_test",
            "HVKM7CXJ",
            vec![Status::Added(ProjectRelativePath::new(
                "src/generated/new_module.rs",
            ))],
        )
        .await;
    }
}
