use common::errors::MegaError;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement,
    TransactionError, TransactionTrait,
};
use std::{path::Path, time::Duration};
use tracing::log;

use common::config::DbConfig;

use crate::utils::id_generator;

/// Create a database connection.
/// When postgres is set but not available, it will fall back to sqlite automatically.
pub async fn database_connection(db_config: &DbConfig) -> DatabaseConnection {
    id_generator::set_up_options().unwrap();

    if db_config.db_type == "postgres" {
        match postgres_connection(db_config).await {
            Ok(conn) => {
                Migrator::up(&conn, None).await.unwrap();
                conn
            }
            Err(e) => {
                log::error!("Failed to connect to postgres: {}", e);
                log::info!("Falling back to sqlite");
                sqlite_connection(db_config)
                    .await
                    .expect("Cannot connect to any database")
            }
        }
    } else {
        sqlite_connection(db_config).await.unwrap()
    }
}

async fn postgres_connection(db_config: &DbConfig) -> Result<DatabaseConnection, MegaError> {
    let db_url = db_config.db_url.to_owned();
    log::info!("Connecting to database: {}", db_url);

    let opt = setup_option(db_url);
    Database::connect(opt).await.map_err(|e| e.into())
}

async fn sqlite_connection(db_config: &DbConfig) -> Result<DatabaseConnection, MegaError> {
    if !Path::new(&db_config.db_path).exists() {
        eprintln!("Creating new sqlite database: {:?}", db_config.db_path);
        std::fs::create_dir_all(Path::new(&db_config.db_path).parent().unwrap())?;
        std::fs::File::create(&db_config.db_path)?;
    }
    let db_url = format!("sqlite://{}", db_config.db_path.to_string_lossy());
    log::info!("Connecting to database: {}", db_url);

    let opt = setup_option(db_url);
    let conn = Database::connect(opt).await?;

    // setup sqlite database (execute .sql)
    if db_config.db_path.metadata()?.len() == 0 {
        log::info!("Setting up sqlite database");
        setup_sql(&conn)
            .await
            .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))?;
    }
    Ok(conn)
}

/// create table from .sql file
async fn setup_sql(conn: &DatabaseConnection) -> Result<(), TransactionError<DbErr>> {
    conn.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            let backend = txn.get_database_backend();

            // `include_str!` will expand the file while compiling, so `.sql` is not needed after that
            const SETUP_SQL: &str = include_str!("../../sqlite_20241204_init.sql");
            txn.execute(Statement::from_string(backend, SETUP_SQL))
                .await?;
            Ok(())
        })
    })
    .await
}

fn setup_option(db_url: impl Into<String>) -> ConnectOptions {
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);
    opt
}
