pub mod base_storage;
pub mod buck_storage;
pub mod build_trigger_storage;
pub mod cl_reviewer_storage;
pub mod cl_storage;
pub mod code_review_comment_storage;
pub mod code_review_thread_storage;
pub mod commit_binding_storage;
pub mod conversation_storage;
pub mod dynamic_sidebar_storage;
pub mod git_db_storage;
pub mod gpg_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod merge_queue_storage;
pub mod mono_storage;
pub mod note_storage;
pub mod stg_common;
pub mod user_storage;
pub mod vault_storage;

use std::sync::{Arc, LazyLock, Weak};

use common::{config::Config, errors::MegaError};
use io_orbit::factory::ObjectStorageFactory;
use tokio::sync::Semaphore;

use crate::{
    service::{
        buck_service::BuckService, cl_service::CLService, code_review_service::CodeReviewService,
        git_service::GitService, import_service::ImportService, issue_service::IssueService,
        lfs_service::LfsService, merge_queue_service::MergeQueueService, mono_service::MonoService,
    },
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        buck_storage::BuckStorage,
        build_trigger_storage::BuildTriggerStorage,
        cl_reviewer_storage::ClReviewerStorage,
        cl_storage::ClStorage,
        code_review_comment_storage::CodeReviewCommentStorage,
        code_review_thread_storage::CodeReviewThreadStorage,
        commit_binding_storage::CommitBindingStorage,
        conversation_storage::ConversationStorage,
        dynamic_sidebar_storage::DynamicSidebarStorage,
        git_db_storage::GitDbStorage,
        gpg_storage::GpgStorage,
        init::database_connection,
        issue_storage::IssueStorage,
        lfs_db_storage::LfsDbStorage,
        merge_queue_storage::MergeQueueStorage,
        mono_storage::MonoStorage,
        note_storage::NoteStorage,
        user_storage::UserStorage,
        vault_storage::VaultStorage,
    },
};

#[derive(Clone)]
pub struct AppService {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub gpg_storage: GpgStorage,
    pub lfs_db_storage: LfsDbStorage,
    pub user_storage: UserStorage,
    pub vault_storage: VaultStorage,
    pub cl_storage: ClStorage,
    pub issue_storage: IssueStorage,
    pub conversation_storage: ConversationStorage,
    pub note_storage: NoteStorage,
    pub commit_binding_storage: CommitBindingStorage,
    pub reviewer_storage: ClReviewerStorage,
    pub merge_queue_storage: MergeQueueStorage,
    pub buck_storage: BuckStorage,
    pub dynamic_sidebar_storage: DynamicSidebarStorage,
    pub code_review_comment_storage: CodeReviewCommentStorage,
    pub code_review_thread_storage: CodeReviewThreadStorage,
    pub build_trigger_storage: BuildTriggerStorage,
}

impl AppService {
    fn mock() -> Arc<Self> {
        let mock = BaseStorage::mock();
        // For tests and in-memory workflows we don't need a real persistent
        // object storage. Use a filesystem-backed storage rooted in the system
        // temp directory to provide a lightweight implementation.
        Arc::new(Self {
            mono_storage: MonoStorage { base: mock.clone() },
            git_db_storage: GitDbStorage { base: mock.clone() },
            gpg_storage: GpgStorage { base: mock.clone() },
            lfs_db_storage: LfsDbStorage { base: mock.clone() },
            user_storage: UserStorage { base: mock.clone() },
            vault_storage: VaultStorage { base: mock.clone() },
            cl_storage: ClStorage { base: mock.clone() },
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
            note_storage: NoteStorage { base: mock.clone() },
            commit_binding_storage: CommitBindingStorage { base: mock.clone() },
            reviewer_storage: ClReviewerStorage { base: mock.clone() },
            merge_queue_storage: MergeQueueStorage::new(mock.clone()),
            buck_storage: BuckStorage { base: mock.clone() },
            dynamic_sidebar_storage: DynamicSidebarStorage { base: mock.clone() },
            code_review_comment_storage: CodeReviewCommentStorage { base: mock.clone() },
            code_review_thread_storage: CodeReviewThreadStorage { base: mock.clone() },
            build_trigger_storage: BuildTriggerStorage { base: mock.clone() },
        })
    }
}

#[derive(Clone)]
pub struct Storage {
    pub(crate) app_service: Arc<AppService>,
    pub issue_service: IssueService,
    pub cl_service: CLService,
    pub merge_queue_service: MergeQueueService,
    pub buck_service: BuckService,
    pub mono_service: MonoService,
    pub import_service: ImportService,
    pub git_service: GitService,
    pub lfs_service: LfsService,
    pub config: Weak<Config>,
    pub code_review_service: CodeReviewService,
}

impl Storage {
    pub async fn new(config: Arc<Config>) -> Result<Self, MegaError> {
        let connection = Arc::new(database_connection(&config.database).await);
        let base = BaseStorage::new(connection.clone());

        let mono_storage = MonoStorage { base: base.clone() };
        let git_db_storage = GitDbStorage { base: base.clone() };
        let gpg_storage = GpgStorage { base: base.clone() };
        let lfs_db_storage = LfsDbStorage { base: base.clone() };
        let user_storage = UserStorage { base: base.clone() };
        let cl_storage = ClStorage { base: base.clone() };
        let issue_storage = IssueStorage { base: base.clone() };
        let vault_storage = VaultStorage { base: base.clone() };
        let conversation_storage = ConversationStorage { base: base.clone() };
        // Initialize LfsService for LFS using the same storage type as configured
        let lfs_service = LfsService {
            lfs_storage: lfs_db_storage.clone(),
            obj_storage: ObjectStorageFactory::build(
                config.lfs.storage_type,
                &config.object_storage,
            )
            .await?,
        };

        let note_storage = NoteStorage { base: base.clone() };
        let commit_binding_storage = CommitBindingStorage { base: base.clone() };
        let reviewer_storage = ClReviewerStorage { base: base.clone() };
        let merge_queue_storage = MergeQueueStorage::new(base.clone());
        let buck_storage = BuckStorage { base: base.clone() };
        let dynamic_sidebar_storage = DynamicSidebarStorage { base: base.clone() };
        let code_review_comment_storage = CodeReviewCommentStorage { base: base.clone() };
        let code_review_thread_storage = CodeReviewThreadStorage { base: base.clone() };
        let build_trigger_storage = BuildTriggerStorage { base: base.clone() };

        let git_service = GitService {
            obj_storage: ObjectStorageFactory::build(
                config.monorepo.storage_type,
                &config.object_storage,
            )
            .await?,
        };
        let mono_service = MonoService {
            mono_storage: mono_storage.clone(),
            git_service: git_service.clone(),
        };

        let import_service = ImportService {
            git_db_storage: git_db_storage.clone(),
            git_service: git_service.clone(),
        };

        let buck_config = config.buck.clone().unwrap_or_default();

        // Validate configuration
        if let Err(e) = buck_config.validate() {
            let error_msg = format!(
                "Invalid Buck configuration: {}. Service cannot start with invalid configuration.",
                e
            );
            tracing::error!("{}", error_msg);
            panic!("{}", error_msg);
        }

        let upload_semaphore = Arc::new(Semaphore::new(
            buck_config.upload_concurrency_limit as usize,
        ));
        let large_file_semaphore = Arc::new(Semaphore::new(
            buck_config.large_file_concurrency_limit as usize,
        ));

        let app_service = AppService {
            mono_storage: mono_storage.clone(),
            git_db_storage,
            gpg_storage,
            lfs_db_storage,
            user_storage,
            vault_storage,
            cl_storage: cl_storage.clone(),
            issue_storage,
            conversation_storage,
            note_storage,
            commit_binding_storage,
            reviewer_storage,
            merge_queue_storage: merge_queue_storage.clone(),
            buck_storage,
            dynamic_sidebar_storage,
            code_review_comment_storage,
            code_review_thread_storage,
            build_trigger_storage,
        };
        let merge_queue_service = MergeQueueService::new(base.clone());
        let buck_service = BuckService::new(
            base.clone(),
            CLService::new(base.clone()),
            upload_semaphore,
            large_file_semaphore,
            buck_config,
            git_service.clone(),
        )
        .expect("failed to create BuckService");

        Ok(Storage {
            app_service: app_service.into(),
            config: Arc::downgrade(&config),
            issue_service: IssueService::new(base.clone()),
            cl_service: CLService::new(base.clone()),
            merge_queue_service,
            buck_service,
            git_service,
            mono_service,
            import_service,
            lfs_service,
            code_review_service: CodeReviewService::new(base.clone()),
        })
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.upgrade().expect("Config has been dropped")
    }

    /// Get recommended concurrency limit for batch database operations.
    ///
    /// Calculates 50% of max_connection, bounded between 4 and max_connection.
    pub fn get_recommended_batch_concurrency(&self) -> usize {
        let max_conn = self.config().database.max_connection as usize;

        // Handle edge case where config might be 0 or invalid
        let safe_conn = if max_conn == 0 { 16 } else { max_conn };

        // Internal calculation using pure function
        calculate_db_concurrency_limit(safe_conn, 50, 4)
    }

    pub fn mono_storage(&self) -> MonoStorage {
        self.app_service.mono_storage.clone()
    }

    pub fn git_db_storage(&self) -> GitDbStorage {
        self.app_service.git_db_storage.clone()
    }

    pub fn gpg_storage(&self) -> GpgStorage {
        self.app_service.gpg_storage.clone()
    }

    pub fn lfs_db_storage(&self) -> LfsDbStorage {
        self.app_service.lfs_db_storage.clone()
    }

    pub fn user_storage(&self) -> UserStorage {
        self.app_service.user_storage.clone()
    }

    pub fn vault_storage(&self) -> VaultStorage {
        self.app_service.vault_storage.clone()
    }

    pub fn cl_storage(&self) -> ClStorage {
        self.app_service.cl_storage.clone()
    }

    pub fn issue_storage(&self) -> IssueStorage {
        self.app_service.issue_storage.clone()
    }

    pub fn conversation_storage(&self) -> ConversationStorage {
        self.app_service.conversation_storage.clone()
    }

    pub fn note_storage(&self) -> NoteStorage {
        self.app_service.note_storage.clone()
    }

    pub fn commit_binding_storage(&self) -> CommitBindingStorage {
        self.app_service.commit_binding_storage.clone()
    }

    pub fn reviewer_storage(&self) -> ClReviewerStorage {
        self.app_service.reviewer_storage.clone()
    }

    pub fn merge_queue_storage(&self) -> MergeQueueStorage {
        self.app_service.merge_queue_storage.clone()
    }

    pub fn buck_storage(&self) -> BuckStorage {
        self.app_service.buck_storage.clone()
    }

    pub fn dynamic_sidebar_storage(&self) -> DynamicSidebarStorage {
        self.app_service.dynamic_sidebar_storage.clone()
    }

    pub fn code_review_thread_storage(&self) -> CodeReviewThreadStorage {
        self.app_service.code_review_thread_storage.clone()
    }

    pub fn code_review_comment_storage(&self) -> CodeReviewCommentStorage {
        self.app_service.code_review_comment_storage.clone()
    }

    pub fn build_trigger_storage(&self) -> BuildTriggerStorage {
        self.app_service.build_trigger_storage.clone()
    }

    pub fn mock() -> Self {
        // During test time, we don't need a AppContext,
        // Put config in a leaked static variable thus the weak reference will always be valid.
        static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());

        Storage {
            app_service: AppService::mock(),
            issue_service: IssueService::mock(),
            cl_service: CLService::mock(),
            merge_queue_service: MergeQueueService::mock(),
            buck_service: BuckService::mock(),
            config: Arc::downgrade(&*CONFIG),
            git_service: GitService::mock(),
            mono_service: MonoService::mock(),
            import_service: ImportService::mock(),
            lfs_service: LfsService::mock(),
            code_review_service: CodeReviewService::mock(),
        }
    }
}

// Private helper function for concurrency calculation
// This is a pure function for easy testing and potential reuse
fn calculate_db_concurrency_limit(
    max_connections: usize,
    percentage: usize,
    min_limit: usize,
) -> usize {
    let calculated = (max_connections * percentage) / 100;
    calculated.max(min_limit).min(max_connections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_db_concurrency_limit() {
        // Normal case: 50% of 16 = 8
        assert_eq!(calculate_db_concurrency_limit(16, 50, 4), 8);

        // Min bound: 50% of 5 = 2.5 -> clamped to min 4
        assert_eq!(calculate_db_concurrency_limit(5, 50, 4), 4);

        // Max bound: 50% of 100 = 50 (within bounds)
        assert_eq!(calculate_db_concurrency_limit(100, 50, 4), 50);

        // Edge case: small connection pool
        assert_eq!(calculate_db_concurrency_limit(4, 50, 4), 4);

        // Edge case: very small connection pool (cannot exceed max_connections)
        assert_eq!(calculate_db_concurrency_limit(2, 50, 4), 2);
    }

    #[test]
    fn test_get_recommended_batch_concurrency() {
        // Create a mock Storage for testing
        let storage = Storage::mock();

        // The mock config should have default max_connection = 16
        let concurrency = storage.get_recommended_batch_concurrency();

        // Should be 50% of 16 = 8
        assert_eq!(concurrency, 8);
    }
}
