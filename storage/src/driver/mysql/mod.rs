pub mod storage;

use std::{env, time::Duration};

use sea_orm::{ConnectOptions, Database};
use tracing::log;

use self::storage::MysqlStorage;

pub async fn init() -> MysqlStorage {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let mut opt = ConnectOptions::new(db_url.to_owned());
    // max_connections is properly for double size of the cpu core
    opt.max_connections(32)
        .min_connections(16)
        .acquire_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);
    MysqlStorage::new(
        Database::connect(opt)
            .await
            .expect("Database connection failed"),
    )
}
