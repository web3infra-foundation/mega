use std::{env, time::Duration};

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log;

use crate::utils::id_generator;

pub async fn database_connection() -> DatabaseConnection {
    id_generator::set_up_options().unwrap();

    let db_url = env::var("MEGA_DB_POSTGRESQL_URL").expect("DATABASE_URL is not set in .env file");
    let max_connections = env::var("MEGA_DB_MAX_CONNECTIONS")
        .expect("MEGA_DB_MAX_CONNECTIONS not configured")
        .parse::<u32>()
        .unwrap();
    let min_connections = env::var("MEGA_DB_MIN_CONNECTIONS")
        .expect("MEGA_DB_MAX_CONNECTIONS not configured")
        .parse::<u32>()
        .unwrap();
    let mut opt = ConnectOptions::new(db_url.to_owned());
    // max_connections is properly for double size of the cpu core
    opt.max_connections(max_connections)
        .min_connections(min_connections)
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
        .sqlx_logging_level(log::LevelFilter::Debug);
    Database::connect(opt)
        .await
        .expect("Database connection failed")
}
