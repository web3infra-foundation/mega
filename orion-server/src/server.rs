use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use chrono::{FixedOffset, Utc};
use http::{HeaderValue, Method};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, Schema, TransactionTrait,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::{self, AppState};
use crate::model::tasks;
/// OpenAPI documentation configuration
#[derive(OpenApi)]
#[openapi(
    paths(
        api::task_handler,
        api::task_build_handler,
        api::task_status_handler,
        api::task_build_list_handler,
        api::task_output_handler,
        api::task_history_output_handler,
        api::task_query_by_mr,
        api::tasks_handler,
    ),
    components(
        schemas(
            crate::scheduler::BuildRequest,
            crate::scheduler::LogSegment,
            api::TaskStatus,
            api::TaskStatusEnum,
            api::TaskDTO,
            api::TaskInfoDTO

        )
    ),
    tags(
        (name = "Build", description = "Build related endpoints")
    )
)]
pub struct ApiDoc;

/// Starts the Orion server with the specified port
/// Initializes database connection, sets up routes, and starts health check tasks
pub async fn start_server(port: u16) {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");
    setup_tables(&conn).await.expect("Failed to setup tables");

    let state = AppState::new(conn, None);

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

/// Sets up database tables if they don't exist
async fn setup_tables(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let trans = conn.begin().await?;
    let builder = conn.get_database_backend();
    let schema = Schema::new(builder);
    let statement = builder.build(
        schema
            .create_table_from_entity(tasks::Entity)
            .if_not_exists(),
    );
    trans.execute(statement).await?;
    trans.commit().await
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
                if let crate::scheduler::WorkerStatus::Busy(task_id) = worker_info.status {
                    tracing::warn!(
                        "Worker {} was busy with task {}. Marking task as Interrupted.",
                        worker_id,
                        task_id
                    );
                    state.scheduler.active_builds.remove(&task_id);

                    let update_res = tasks::Entity::update_many()
                        .set(tasks::ActiveModel {
                            end_at: Set(Some(
                                Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                            )),
                            ..Default::default()
                        })
                        .filter(tasks::Column::TaskId.eq(task_id.parse::<uuid::Uuid>().unwrap()))
                        .exec(&state.conn)
                        .await;

                    if let Err(e) = update_res {
                        tracing::error!("Failed to update orphaned task {} in DB: {}", task_id, e);
                    }
                }
            }
        }
    }
}
