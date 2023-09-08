use clap::ValueEnum;
use driver::{mysql::storage::MysqlStorage, postgres::storage::PgStorage, ObjectStorage};

pub mod driver;
pub mod utils;
use std::{env, sync::Arc, time::Duration};

use sea_orm::{ConnectOptions, Database};
use tracing::log;

use crate::utils::id_generator;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum DataSource {
    Mysql,
    Postgres,
}

pub async fn init(data_source: &DataSource) -> Arc<dyn ObjectStorage> {
    id_generator::set_up_options().unwrap();

    let db_url = match data_source {
        DataSource::Mysql => {
            env::var("MEGA_DB_MYSQL_URL").expect("DATABASE_URL is not set in .env file")
        }
        DataSource::Postgres => {
            env::var("MEGA_DB_POSTGRESQL_URL").expect("DATABASE_URL is not set in .env file")
        }
    };

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
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);
    let connection = Database::connect(opt)
        .await
        .expect("Database connection failed");
    match data_source {
        DataSource::Mysql => Arc::new(MysqlStorage { connection }),
        DataSource::Postgres => Arc::new(PgStorage { connection }),
    }
}
