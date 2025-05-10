use std::env;
use std::time::Duration;

use sea_orm::DatabaseConnection;
use sea_orm::{ConnectOptions, Database};

pub async fn db_connection() -> DatabaseConnection {
    let mut opt = ConnectOptions::new(env::var("MEGA_DB_POSTGRESQL_URL").unwrap());

    // max_connections is properly for double size of the cpu core
    opt.max_connections(16)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(
            env::var("MEGA_DB_SQLX_LOGGING")
                .unwrap()
                .parse::<bool>()
                .unwrap(),
        )
        .sqlx_logging_level(tracing::log::LevelFilter::Debug);
    Database::connect(opt)
        .await
        .expect("Database connection failed")
}
