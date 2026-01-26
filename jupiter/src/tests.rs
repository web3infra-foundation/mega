use std::{
    path::Path,
    sync::{Arc, LazyLock},
};

use common::config::Config;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log;

use crate::{
    migration::apply_migrations,
    service::{
        buck_service::BuckService, cl_service::CLService, code_review_service::CodeReviewService,
        git_service::GitService, import_service::ImportService, issue_service::IssueService,
        lfs_service::LfsService, merge_queue_service::MergeQueueService, mono_service::MonoService,
    },
    storage::{
        AppService, Storage,
        base_storage::{BaseStorage, StorageConnector},
        buck_storage::BuckStorage,
        cl_reviewer_storage::ClReviewerStorage,
        cl_storage::ClStorage,
        code_review_comment_storage::CodeReivewCommentStorage,
        code_review_thread_storage::CodeReviewThreadStorage,
        commit_binding_storage::CommitBindingStorage,
        conversation_storage::ConversationStorage,
        dynamic_sidebar_storage::DynamicSidebarStorage,
        git_db_storage::GitDbStorage,
        gpg_storage::GpgStorage,
        issue_storage::IssueStorage,
        lfs_db_storage::LfsDbStorage,
        merge_queue_storage::MergeQueueStorage,
        mono_storage::MonoStorage,
        note_storage::NoteStorage,
        user_storage::UserStorage,
        vault_storage::VaultStorage,
    },
};

pub async fn test_db_connection(temp_dir: &Path) -> DatabaseConnection {
    let db_url = format!("sqlite://{}/test.db", temp_dir.to_string_lossy());
    std::fs::File::create(temp_dir.join("test.db")).expect("Failed to create test database file");

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
    let connection = test_db_connection(temp_dir.as_ref()).await;
    let connection = Arc::new(connection);
    let config = CONFIG.clone();
    let base = BaseStorage::new(connection.clone());

    let svc = AppService {
        mono_storage: MonoStorage { base: base.clone() },
        git_db_storage: GitDbStorage { base: base.clone() },
        gpg_storage: GpgStorage { base: base.clone() },
        lfs_db_storage: LfsDbStorage { base: base.clone() },
        user_storage: UserStorage { base: base.clone() },
        cl_storage: ClStorage { base: base.clone() },
        issue_storage: IssueStorage { base: base.clone() },
        vault_storage: VaultStorage { base: base.clone() },
        conversation_storage: ConversationStorage { base: base.clone() },
        note_storage: NoteStorage { base: base.clone() },
        commit_binding_storage: CommitBindingStorage { base: base.clone() },
        reviewer_storage: ClReviewerStorage { base: base.clone() },
        merge_queue_storage: MergeQueueStorage::new(base.clone()),
        buck_storage: BuckStorage { base: base.clone() },
        dynamic_sidebar_storage: DynamicSidebarStorage { base: base.clone() },
        code_review_comment_storage: CodeReivewCommentStorage { base: base.clone() },
        code_review_thread_storage: CodeReviewThreadStorage { base: base.clone() },
    };

    apply_migrations(&connection, true).await.unwrap();

    Storage {
        app_service: Arc::new(svc),
        issue_service: IssueService::mock(),
        cl_service: CLService::mock(),
        merge_queue_service: MergeQueueService::mock(),
        buck_service: BuckService::mock(),
        config: Arc::downgrade(&config),
        git_service: GitService::mock(),
        mono_service: MonoService::mock(),
        import_service: ImportService::mock(),
        lfs_service: LfsService::mock(),
        code_review_service: CodeReviewService::mock(),
    }
}
