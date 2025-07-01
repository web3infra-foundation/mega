use crate::api;
use crate::api::AppState;
use crate::model::builds;
use axum::Router;
use axum::routing::get;
use dashmap::DashMap;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, Schema, TransactionTrait};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub async fn start_server(port: u16) {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");

    let conn = Database::connect(db_url) // TODO pool
        .await
        .expect("Database connection failed");
    setup_tables(&conn).await.expect("Failed to setup tables");

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(api::routers())
        .with_state(AppState {
            clients: Arc::new(DashMap::new()),
            conn,
            building: Arc::new(DashMap::new()),
        })
        // logging so we can see what's going on
        .layer(TraceLayer::new_for_http());

    tracing::info!("Listening on port {}", port);

    let addr = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    axum::serve(
        addr,
        app.into_make_service_with_connect_info::<SocketAddr>(), // or `ConnectInfo<SocketAddr>` fail
    )
    .await
    .unwrap();
}

/// create if not exists
async fn setup_tables(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let trans = conn.begin().await?;

    let builder = conn.get_database_backend();
    let schema = Schema::new(builder);
    let mut table_statement = schema.create_table_from_entity(builds::Entity);
    table_statement.if_not_exists();
    let statement = builder.build(&table_statement);
    trans.execute(statement).await?;

    trans.commit().await
}
