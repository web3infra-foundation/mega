use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log;

use common::config::Config;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use crate::lfs_storage::local_storage::LocalStorage;
use crate::migration::apply_migrations;
use crate::service::issue_service::IssueService;
use crate::service::mr_service::MRService;
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::gpg_storage::GpgStorage;
use crate::storage::note_storage::NoteStorage;
use crate::storage::{
    commit_binding_storage::CommitBindingStorage, conversation_storage::ConversationStorage,
    git_db_storage::GitDbStorage, issue_storage::IssueStorage, lfs_db_storage::LfsDbStorage,
    mono_storage::MonoStorage, mr_reviewer_storage::MrReviewerStorage, mr_storage::MrStorage,
    raw_db_storage::RawDbStorage, relay_storage::RelayStorage, user_storage::UserStorage,
    vault_storage::VaultStorage,
};
use crate::storage::{AppService, Storage};

pub async fn test_db_connection(temp_dir: impl AsRef<Path>) -> DatabaseConnection {
    let db_url = format!("sqlite://{}/test.db", temp_dir.as_ref().to_string_lossy());
    std::fs::File::create(temp_dir.as_ref().join("test.db"))
        .expect("Failed to create test database file");

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(5)
        .min_connections(1)
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);

    Database::connect(opt)
        .await
        .expect("Failed to connect to mock database")
}

pub async fn test_storage(temp_dir: impl AsRef<Path>) -> Storage {
    static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());
    let connection = test_db_connection(temp_dir).await;
    let connection = Arc::new(connection);
    let config = CONFIG.clone();
    let base = BaseStorage::new(connection.clone());

    let svc = AppService {
        mono_storage: MonoStorage { base: base.clone() },
        git_db_storage: GitDbStorage { base: base.clone() },
        gpg_storage: GpgStorage { base: base.clone() },
        raw_db_storage: RawDbStorage { base: base.clone() },
        lfs_db_storage: LfsDbStorage { base: base.clone() },
        relay_storage: RelayStorage { base: base.clone() },
        user_storage: UserStorage { base: base.clone() },
        mr_storage: MrStorage { base: base.clone() },
        issue_storage: IssueStorage { base: base.clone() },
        vault_storage: VaultStorage { base: base.clone() },
        conversation_storage: ConversationStorage { base: base.clone() },
        lfs_file_storage: Arc::new(LocalStorage::mock()), // fix it when you really use it.
        note_storage: NoteStorage { base: base.clone() },
        commit_binding_storage: CommitBindingStorage { base: base.clone() },
        reviewer_storage: MrReviewerStorage { base: base.clone() },
    };

    apply_migrations(&connection, true).await.unwrap();

    Storage {
        app_service: Arc::new(svc),
        issue_service: IssueService::mock(),
        mr_service: MRService::mock(),
        config: Arc::downgrade(&config),
    }
}
