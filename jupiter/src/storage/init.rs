use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log;

use common::config::DbConfig;

use crate::utils::id_generator;

pub async fn database_connection(db_config: &DbConfig) -> DatabaseConnection {
    id_generator::set_up_options().unwrap();
    let mut opt = ConnectOptions::new(db_config.db_url.to_owned());
    opt.max_connections(db_config.max_connection)
        .min_connections(db_config.min_connection)
        .acquire_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(db_config.sqlx_logging)
        .sqlx_logging_level(log::LevelFilter::Debug);
    Database::connect(opt)
        .await
        .expect("Database connection failed")
}
