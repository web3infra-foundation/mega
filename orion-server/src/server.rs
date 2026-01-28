use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, routing::get};
use chrono::{FixedOffset, Utc};
use common::{
    config::{Config, ObjectStorageBackend, c::ConfigError},
    errors::MegaError,
};
use http::{HeaderValue, Method};
use io_orbit::factory::ObjectStorageFactory;
use orion::ws::TaskPhase;
use sea_orm::{ActiveValue::Set, ColumnTrait, Database, EntityTrait, QueryFilter};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::{self, AppState},
    log::{
        log_service::LogService,
        store::{LogStore, io_orbit_store::IoOrbitLogStore, local_log_store, noop_log_store},
    },
    model::builds,
};

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
        api::build_retry_handler,
        api::health_check_handler
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

pub async fn init_log_service(config: Config) -> Result<LogService, MegaError> {
    // Read buffer size from environment, defaulting to 4096 if unset or invalid

    let buffer = config.orion_server.as_ref().unwrap().log_stream_buffer;

    let log_store_mode = config
        .orion_server
        .as_ref()
        .unwrap()
        .logger_storage_mode
        .clone();

    let build_log_dir = config.orion_server.as_ref().unwrap().build_log_dir.clone();

    let noop_log_store: Arc<dyn LogStore> = Arc::new(noop_log_store::NoopLogStore::new());

    let (local_log_store, cloud_log_store, cloud_upload_enabled): (
        Arc<dyn LogStore>,
        Arc<dyn LogStore>,
        bool,
    ) = match log_store_mode.as_str() {
        "none" => (noop_log_store.clone(), noop_log_store.clone(), false),
        "local" => (
            Arc::new(local_log_store::LocalLogStore::new(&build_log_dir)),
            //Arc::new(ObjectStorageFactory::build(ObjectStorageBackend::Local,)),
            noop_log_store.clone(),
            false,
        ),
        "mix" => {
            let object_store_wrapper =
                ObjectStorageFactory::build(ObjectStorageBackend::Local, &config.object_storage)
                    .await?;
            (
                Arc::new(local_log_store::LocalLogStore::new(&build_log_dir)),
                Arc::new(IoOrbitLogStore::new(object_store_wrapper)),
                true,
            )
        }
        other => panic!(
            "Unsupported LOGGER_STORAGE_TYPE: {}. Supported values: [local, none, s3]",
            other
        ),
    };

    tracing::info!(
        storage_type = %log_store_mode,
        cloud_upload_enabled,
        build_log_dir = %build_log_dir,
        "Initialized log service storage configuration"
    );

    // Create the LogService
    Ok(LogService::new(
        local_log_store,
        cloud_log_store,
        buffer,
        cloud_upload_enabled,
    ))
}

async fn load_orion_config() -> Result<Config, ConfigError> {
    if let Ok(config_path) = env::var("CONFIG_PATH") {
        return Config::new(&config_path);
    }

    let base_dir = common::config::mega_base();
    if base_dir.exists() {
        let config_path = base_dir
            .to_str()
            .ok_or_else(|| ConfigError::NotFound("Invalid config path".to_string()))?;
        return Config::new(config_path);
    }

    Ok(Config::default())
}

/// Starts the Orion server with the specified port
/// Initializes database connection, sets up routes, and starts health check tasks
pub async fn start_server() {
    let config = load_orion_config().await.unwrap();

    let orion_server_config = config.orion_server.clone().unwrap();
    //let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let conn = Database::connect(orion_server_config.db_url)
        .await
        .expect("Database connection failed");

    let port = orion_server_config.port;

    // Initialize the LogService and spawn a background task to watch logs,
    // then create the application state with the same LogService instance.
    let log_service = init_log_service(config).await.unwrap();
    let log_service_clone = log_service.clone();
    tokio::spawn(async move {
        log_service_clone.watch_logs().await;
    });

    let state = AppState::new(conn, None, log_service);

    // Start background health check task
    tokio::spawn(start_health_check_task(state.clone()));

    // Start queue manager
    tokio::spawn(api::start_queue_manager(state.clone()));

    let origins: Vec<HeaderValue> = orion_server_config
        .allowed_cors_origins
        .split(',')
        .map(|x| x.trim().parse::<HeaderValue>().unwrap())
        .collect();

    // let origins: Vec<HeaderValue> = std::env::var("ALLOWED_CORS_ORIGINS")
    //     .unwrap()
    //     .split(',')
    //     .map(|x| x.trim().parse::<HeaderValue>().unwrap())
    //     .collect();

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

                        if let Err(e) = crate::model::targets::update_state(
                            &state.conn,
                            build_model.target_id,
                            crate::model::targets::TargetState::Interrupted,
                            None,
                            Some(Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())),
                            None,
                        )
                        .await
                        {
                            tracing::error!(
                                "Failed to update target {} to Interrupted: {}",
                                build_model.target_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }
}
