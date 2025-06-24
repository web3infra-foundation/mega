use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tempfile::TempDir;
use tracing::log;

use std::sync::{Arc, LazyLock};

use common::config::Config;

use crate::lfs_storage::local_storage::LocalStorage;
use crate::migrator::apply_migrations;
use crate::storage::{
    git_db_storage::GitDbStorage, issue_storage::IssueStorage, lfs_db_storage::LfsDbStorage,
    mono_storage::MonoStorage, mq_storage::MQStorage, mr_storage::MrStorage,
    raw_db_storage::RawDbStorage, relay_storage::RelayStorage, user_storage::UserStorage,
    vault_storage::VaultStorage,
};
use crate::storage::{Service, Storage};

pub async fn test_db_connection() -> (DatabaseConnection, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let db_url = format!("sqlite://{}/test.db", temp_dir.path().to_string_lossy());
    std::fs::File::create(temp_dir.path().join("test.db"))
        .expect("Failed to create test database file");

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(5)
        .min_connections(1)
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to mock database");

    (db, temp_dir)
}

pub async fn test_storage() -> Storage {
    static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());
    let (connection, _) = test_db_connection().await;
    let connection = Arc::new(connection);
    let lfs_db_storage = LfsDbStorage::new(connection.clone()).await;
    let config = CONFIG.clone();

    let svc = Service {
        mono_storage: MonoStorage::new(connection.clone()).await,
        git_db_storage: GitDbStorage::new(connection.clone()).await,
        raw_db_storage: RawDbStorage::new(connection.clone()).await,
        lfs_db_storage: lfs_db_storage.clone(),
        relay_storage: RelayStorage::new(connection.clone()).await,
        mq_storage: MQStorage::new(connection.clone()).await,
        user_storage: UserStorage::new(connection.clone()).await,
        mr_storage: MrStorage::new(connection.clone()).await,
        issue_storage: IssueStorage::new(connection.clone()).await,
        vault_storage: VaultStorage::new(connection.clone()).await,
        lfs_file_storage: Arc::new(LocalStorage::mock()), // fix it until you really use it.
    };

    apply_migrations(&connection, true).await.unwrap();
    Storage {
        services: Arc::new(svc),
        config: Arc::downgrade(&config),
    }
}
