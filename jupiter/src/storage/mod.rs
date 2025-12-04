pub mod base_storage;
pub mod buck_storage;
pub mod cl_reviewer_storage;
pub mod cl_storage;
pub mod commit_binding_storage;
pub mod conversation_storage;
pub mod git_db_storage;
pub mod gpg_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod merge_queue_storage;
pub mod mono_storage;
pub mod note_storage;
pub mod raw_db_storage;
pub mod relay_storage;
pub mod stg_common;
pub mod user_storage;
pub mod vault_storage;

use std::sync::{Arc, LazyLock, Weak};

use common::config::Config;

use crate::lfs_storage::{self, LfsFileStorage, local_storage::LocalStorage};
use crate::service::cl_service::CLService;
use crate::service::issue_service::IssueService;
use crate::service::merge_queue_service::MergeQueueService;
use crate::storage::conversation_storage::ConversationStorage;
use crate::storage::init::database_connection;
use crate::storage::{
    buck_storage::BuckStorage, cl_storage::ClStorage, commit_binding_storage::CommitBindingStorage,
    git_db_storage::GitDbStorage, gpg_storage::GpgStorage, issue_storage::IssueStorage,
    lfs_db_storage::LfsDbStorage, merge_queue_storage::MergeQueueStorage,
    mono_storage::MonoStorage, raw_db_storage::RawDbStorage, relay_storage::RelayStorage,
    user_storage::UserStorage, vault_storage::VaultStorage,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::cl_reviewer_storage::ClReviewerStorage;
use crate::storage::note_storage::NoteStorage;

#[derive(Clone)]
pub struct AppService {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub gpg_storage: GpgStorage,
    pub raw_db_storage: RawDbStorage,
    pub lfs_db_storage: LfsDbStorage,
    pub relay_storage: RelayStorage,
    pub user_storage: UserStorage,
    pub vault_storage: VaultStorage,
    pub cl_storage: ClStorage,
    pub issue_storage: IssueStorage,
    pub conversation_storage: ConversationStorage,
    pub lfs_file_storage: Arc<dyn LfsFileStorage>,
    pub note_storage: NoteStorage,
    pub commit_binding_storage: CommitBindingStorage,
    pub reviewer_storage: ClReviewerStorage,
    pub merge_queue_storage: MergeQueueStorage,
    pub buck_storage: BuckStorage,
}

impl AppService {
    fn mock() -> Arc<Self> {
        let mock = BaseStorage::mock();
        Arc::new(Self {
            mono_storage: MonoStorage { base: mock.clone() },
            git_db_storage: GitDbStorage { base: mock.clone() },
            gpg_storage: GpgStorage { base: mock.clone() },
            raw_db_storage: RawDbStorage { base: mock.clone() },
            lfs_db_storage: LfsDbStorage { base: mock.clone() },
            relay_storage: RelayStorage { base: mock.clone() },
            user_storage: UserStorage { base: mock.clone() },
            vault_storage: VaultStorage { base: mock.clone() },
            lfs_file_storage: Arc::new(LocalStorage::mock()),
            cl_storage: ClStorage { base: mock.clone() },
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
            note_storage: NoteStorage { base: mock.clone() },
            commit_binding_storage: CommitBindingStorage { base: mock.clone() },
            reviewer_storage: ClReviewerStorage { base: mock.clone() },
            merge_queue_storage: MergeQueueStorage::new(mock.clone()),
            buck_storage: BuckStorage { base: mock.clone() },
        })
    }
}

#[derive(Clone)]
pub struct Storage {
    pub(crate) app_service: Arc<AppService>,
    pub issue_service: IssueService,
    pub cl_service: CLService,
    pub merge_queue_service: MergeQueueService,
    pub config: Weak<Config>,
}

impl Storage {
    pub async fn new(config: Arc<Config>) -> Self {
        let connection = Arc::new(database_connection(&config.database).await);
        let base = BaseStorage::new(connection.clone());

        let mono_storage = MonoStorage { base: base.clone() };
        let git_db_storage = GitDbStorage { base: base.clone() };
        let gpg_storage = GpgStorage { base: base.clone() };
        let raw_db_storage = RawDbStorage { base: base.clone() };
        let lfs_db_storage = LfsDbStorage { base: base.clone() };
        let relay_storage = RelayStorage { base: base.clone() };
        let user_storage = UserStorage { base: base.clone() };
        let cl_storage = ClStorage { base: base.clone() };
        let issue_storage = IssueStorage { base: base.clone() };
        let vault_storage = VaultStorage { base: base.clone() };
        let conversation_storage = ConversationStorage { base: base.clone() };
        let lfs_file_storage = lfs_storage::init(config.lfs.clone(), connection.clone()).await;
        let note_storage = NoteStorage { base: base.clone() };
        let commit_binding_storage = CommitBindingStorage { base: base.clone() };
        let reviewer_storage = ClReviewerStorage { base: base.clone() };
        let merge_queue_storage = MergeQueueStorage::new(base.clone());
        let buck_storage = BuckStorage { base: base.clone() };

        let app_service = AppService {
            mono_storage: mono_storage.clone(),
            git_db_storage,
            gpg_storage,
            raw_db_storage,
            lfs_db_storage,
            relay_storage,
            user_storage,
            vault_storage,
            cl_storage: cl_storage.clone(),
            issue_storage,
            conversation_storage,
            lfs_file_storage,
            note_storage,
            commit_binding_storage,
            reviewer_storage,
            merge_queue_storage: merge_queue_storage.clone(),
            buck_storage,
        };
        let merge_queue_service = MergeQueueService::new(base.clone());

        Storage {
            app_service: app_service.into(),
            config: Arc::downgrade(&config),
            issue_service: IssueService::new(base.clone()),
            cl_service: CLService::new(base.clone()),
            merge_queue_service,
        }
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.upgrade().expect("Config has been dropped")
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

    pub fn raw_db_storage(&self) -> RawDbStorage {
        self.app_service.raw_db_storage.clone()
    }

    pub fn lfs_db_storage(&self) -> LfsDbStorage {
        self.app_service.lfs_db_storage.clone()
    }

    pub fn relay_storage(&self) -> RelayStorage {
        self.app_service.relay_storage.clone()
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

    pub fn lfs_file_storage(&self) -> Arc<dyn LfsFileStorage> {
        self.app_service.lfs_file_storage.clone()
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

    pub fn mock() -> Self {
        // During test time, we don't need a AppContext,
        // Put config in a leaked static variable thus the weak reference will always be valid.
        static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());

        Storage {
            app_service: AppService::mock(),
            issue_service: IssueService::mock(),
            cl_service: CLService::mock(),
            merge_queue_service: MergeQueueService::mock(),
            config: Arc::downgrade(&*CONFIG),
        }
    }
}
