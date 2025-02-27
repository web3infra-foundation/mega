use crate::api;
use crate::model::builds;
use axum::routing::get;
use axum::Router;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, Schema, TransactionTrait};

#[derive(Clone)]
pub struct AppState {
    pub(crate) conn: DatabaseConnection,
}

pub async fn start_server(port: u16) {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");

    let conn = Database::connect(db_url) // TODO pool
        .await
        .expect("Database connection failed");
    setup_tables(&conn).await.expect("Failed to setup tables");

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(api::routers())
        .with_state(AppState { conn });

    tracing::info!("Listening on port {}", port);

    let addr = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(addr, app.into_make_service()).await.unwrap();
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
