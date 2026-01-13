use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use chrono::{FixedOffset, Utc};
use http::{HeaderValue, Method};
use orion::ws::TaskPhase;
use sea_orm::{ActiveValue::Set, ColumnTrait, Database, EntityTrait, QueryFilter};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::{self, AppState};
use crate::log::log_service::LogService;
use crate::log::store::{LogStore, local_log_store, noop_log_store, s3_log_store};
use crate::model::builds;
/// OpenAPI documentation configuration
#[derive(OpenApi)]
#[openapi(
    paths(
        api::task_handler,
        api::task_build_list_handler,
        api::task_output_handler,
        api::task_history_output_handler,
        api::target_logs_handler,
        api::tasks_handler,
        api::task_targets_handler,
        api::get_orion_clients_info,
        api::get_orion_client_status_by_id,
        api::build_retry_handler
    ),
    components(
        schemas(
            crate::scheduler::BuildRequest,
            crate::scheduler::LogSegment,
            api::TaskStatusEnum,
            api::BuildDTO,
            api::TargetDTO,
            api::TargetLogQuery,
            api::TaskInfoDTO,
            api::OrionClientInfo,
            api::OrionClientStatus,
            api::CoreWorkerStatus,
            api::OrionClientQuery,
            crate::model::targets::TargetState,
            TaskPhase,
        )
    ),
    tags(
        (name = "Build", description = "Build related endpoints")
    )
)]
pub struct ApiDoc;

pub async fn init_log_service() -> LogService {
    // Read buffer size from environment, defaulting to 4096 if unset or invalid
    let buffer = std::env::var("LOG_STREAM_BUFFER")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(4096);

    // Read log store type and bucket/build dir
    let log_store_type =
        std::env::var("LOGGER_STORAGE_TYPE").unwrap_or_else(|_| "local".to_string());
    let bucket_name = std::env::var("BUCKET_NAME").unwrap_or_else(|_| "default-bucket".to_string());
    let build_log_dir = std::env::var("BUILD_LOG_DIR").unwrap_or_else(|_| "/tmp/logs".to_string());

    let noop_log_store: Arc<dyn LogStore> = Arc::new(noop_log_store::NoopLogStore::new());

    let (local_log_store, cloud_log_store, cloud_upload_enabled): (
        Arc<dyn LogStore>,
        Arc<dyn LogStore>,
        bool,
    ) = match log_store_type.as_str() {
        "none" => (noop_log_store.clone(), noop_log_store.clone(), false),
        "local" => (
            Arc::new(local_log_store::LocalLogStore::new(&build_log_dir)),
            noop_log_store.clone(),
            false,
        ),
        "s3" => {
            let access_key =
                std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set for S3");
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
                .expect("AWS_SECRET_ACCESS_KEY must be set for S3");
            let region =
                std::env::var("AWS_DEFAULT_REGION").expect("AWS_DEFAULT_REGION must be set for S3");

            let store =
                s3_log_store::S3LogStore::new(&bucket_name, &region, &access_key, &secret_key)
                    .await;
            (
                Arc::new(local_log_store::LocalLogStore::new(&build_log_dir)),
                Arc::new(store),
                true,
            )
        }
        other => panic!(
            "Unsupported LOGGER_STORAGE_TYPE: {}. Supported values: [local, none, s3]",
            other
        ),
    };

    tracing::info!(
        storage_type = %log_store_type,
        cloud_upload_enabled,
        build_log_dir = %build_log_dir,
        "Initialized log service storage configuration"
    );

    // Create the LogService
    LogService::new(
        local_log_store,
        cloud_log_store,
        buffer,
        cloud_upload_enabled,
    )
}

/// Starts the Orion server with the specified port
/// Initializes database connection, sets up routes, and starts health check tasks
pub async fn start_server(port: u16) {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");

    // Initialize the LogService and spawn a background task to watch logs,
    // then create the application state with the same LogService instance.
    let log_service = init_log_service().await;
    let log_service_clone = log_service.clone();
    tokio::spawn(async move {
        log_service_clone.watch_logs().await;
    });

    let state = AppState::new(conn, None, log_service);

    // Start background health check task
    tokio::spawn(start_health_check_task(state.clone()));

    // Start queue manager
    tokio::spawn(api::start_queue_manager(state.clone()));

    let origins: Vec<HeaderValue> = std::env::var("ALLOWED_CORS_ORIGINS")
        .unwrap()
        .split(',')
        .map(|x| x.trim().parse::<HeaderValue>().unwrap())
        .collect();

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(api::routers())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::CONTENT_TYPE,
                    ])
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::OPTIONS,
                        Method::DELETE,
                        Method::PUT,
                    ])
                    .allow_credentials(true),
            ),
        );

    tracing::info!("Listening on port {}", port);
    let addr = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    axum::serve(
        addr,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// Background task that monitors worker health and handles timeouts
/// Removes dead workers and marks their tasks as interrupted
async fn start_health_check_task(state: AppState) {
    let health_check_interval = Duration::from_secs(30);
    let worker_timeout = Duration::from_secs(90);

    tracing::info!(
        "Health check task started. Interval: {:?}, Worker timeout: {:?}",
        health_check_interval,
        worker_timeout
    );

    loop {
        tokio::time::sleep(health_check_interval).await;
        tracing::debug!("Running health check...");

        let mut dead_workers = Vec::new();
        let now = chrono::Utc::now();

        // Find workers that haven't sent heartbeat within timeout period
        for entry in state.scheduler.workers.iter() {
            if now.signed_duration_since(entry.value().last_heartbeat)
                > chrono::Duration::from_std(worker_timeout).unwrap()
            {
                dead_workers.push(entry.key().clone());
            }
        }

        if dead_workers.is_empty() {
            continue;
        }

        tracing::warn!("Found dead workers: {:?}", dead_workers);

        // Remove dead workers and handle their tasks
        for worker_id in dead_workers {
            if let Some((_, worker_info)) = state.scheduler.workers.remove(&worker_id) {
                tracing::info!("Removed dead worker: {}", worker_id);

                // If worker was busy, mark task as interrupted
                if let crate::scheduler::WorkerStatus::Busy { task_id, .. } = worker_info.status {
                    tracing::warn!(
                        "Worker {} was busy with task {}. Marking task as Interrupted.",
                        worker_id,
                        task_id
                    );
                    state.scheduler.active_builds.remove(&task_id);

                    let build_uuid = match task_id.parse::<uuid::Uuid>() {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            tracing::warn!("Invalid build id {} when marking interrupted", task_id);
                            continue;
                        }
                    };

                    if let Ok(Some(build_model)) = builds::Entity::find_by_id(build_uuid)
                        .one(&state.conn)
                        .await
                    {
                        let update_res = builds::Entity::update_many()
                            .set(builds::ActiveModel {
                                end_at: Set(Some(
                                    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                                )),
                                exit_code: Set(None),
                                ..Default::default()
                            })
                            .filter(builds::Column::Id.eq(build_uuid))
                            .exec(&state.conn)
                            .await;

                        if let Err(e) = update_res {
                            tracing::error!(
                                "Failed to update orphaned task {} in DB: {}",
                                task_id,
                                e
                            );
                        }

                        let _ = crate::model::targets::update_state(
                            &state.conn,
                            build_model.target_id,
                            crate::model::targets::TargetState::Interrupted,
                            None,
                            Some(Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())),
                            None,
                        )
                        .await
                        .map_err(|e| tracing::warn!("update target state failed: {e}"));
                    }
                }
            }
        }
    }
}
