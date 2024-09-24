use std::{path::Path, time::Duration};

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement, TransactionError, TransactionTrait};
use tracing::log;

use common::config::DbConfig;

use crate::utils::id_generator;

pub async fn database_connection(db_config: &DbConfig) -> DatabaseConnection {
    id_generator::set_up_options().unwrap();

    let is_sqlite = db_config.db_type == "sqlite";
    let db_path = &db_config.db_path;
    let db_url = if is_sqlite {
        if !Path::new(db_path).exists() {
            log::info!("Creating new sqlite database: {}", db_path);
            std::fs::File::create(db_path).expect("Failed to create sqlite database");
        }
        &format!("sqlite://{}", db_path)
    } else {
        &db_config.db_url
    };
    log::info!("Connecting to database: {}", db_url);

    let mut opt = ConnectOptions::new(db_url.to_owned());
    opt.max_connections(db_config.max_connection)
        .min_connections(db_config.min_connection)
        .acquire_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(db_config.sqlx_logging)
        .sqlx_logging_level(log::LevelFilter::Debug);
    let conn = Database::connect(opt)
        .await
        .expect("Database connection failed");

    // setup sqlite database (execute .sql)
    if is_sqlite && is_file_empty(db_path) {
        log::info!("Setting up sqlite database");
        setup_sql(&conn).await.expect("Failed to setup sqlite database");
    }
    conn
}

/// create table from .sql file
async fn setup_sql(conn: &DatabaseConnection) -> Result<(), TransactionError<DbErr>> {
    conn.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            let backend = txn.get_database_backend();

            // `include_str!` will expand the file while compiling, so `.sql` is not needed after that
            const SETUP_SQL: &str = include_str!("../../../sql/sqlite/sqlite_20240923_init.sql");
            txn.execute(Statement::from_string(backend, SETUP_SQL)).await?;
            Ok(())
        })
    })
    .await
}

fn is_file_empty(path: &str) -> bool {
    let metadata = std::fs::metadata(path).unwrap();
    metadata.len() == 0
}