pub mod base_storage;
pub mod conversation_storage;
pub mod git_db_storage;
pub mod gpg_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod mono_storage;
pub mod mr_storage;
pub mod note_storage;
pub mod raw_db_storage;
pub mod relay_storage;
pub mod stg_common;
pub mod user_storage;
pub mod vault_storage;

use std::sync::{Arc, LazyLock, Weak};

use common::config::Config;

use crate::lfs_storage::{self, local_storage::LocalStorage, LfsFileStorage};
use crate::service::issue_service::IssueService;
use crate::service::mr_service::MRService;
use crate::storage::conversation_storage::ConversationStorage;
use crate::storage::init::database_connection;
use crate::storage::{
    git_db_storage::GitDbStorage, issue_storage::IssueStorage, lfs_db_storage::LfsDbStorage,
    mono_storage::MonoStorage, mr_storage::MrStorage, raw_db_storage::RawDbStorage,
    relay_storage::RelayStorage, user_storage::UserStorage, vault_storage::VaultStorage,
    gpg_storage::GpgStorage
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};
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
    pub mr_storage: MrStorage,
    pub issue_storage: IssueStorage,
    pub conversation_storage: ConversationStorage,
    pub lfs_file_storage: Arc<dyn LfsFileStorage>,
    pub note_storage: NoteStorage,
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
            mr_storage: MrStorage { base: mock.clone() },
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
            note_storage: NoteStorage { base: mock.clone() },
        })
    }
}

#[derive(Clone)]
pub struct Storage {
    pub(crate) app_service: Arc<AppService>,
    pub issue_service: IssueService,
    pub mr_service: MRService,
    pub config: Weak<Config>,
}

impl Storage {
    pub async fn new(config: Arc<Config>) -> Self {
        let connection = Arc::new(database_connection(&config.database).await);
        let base = BaseStorage::new(connection.clone());

        let mono_storage = MonoStorage { base: base.clone() };
        let git_db_storage = GitDbStorage { base: base.clone() };
        let gpg_storage = GpgStorage {base: base.clone()};
        let raw_db_storage = RawDbStorage { base: base.clone() };
        let lfs_db_storage = LfsDbStorage { base: base.clone() };
        let relay_storage = RelayStorage { base: base.clone() };
        let user_storage = UserStorage { base: base.clone() };
        let mr_storage = MrStorage { base: base.clone() };
        let issue_storage = IssueStorage { base: base.clone() };
        let vault_storage = VaultStorage { base: base.clone() };
        let conversation_storage = ConversationStorage { base: base.clone() };
        let lfs_file_storage = lfs_storage::init(config.lfs.clone(), connection.clone()).await;
        let note_storage = NoteStorage { base: base.clone() };

        let app_service = AppService {
            mono_storage,
            git_db_storage,
            gpg_storage,
            raw_db_storage,
            lfs_db_storage,
            relay_storage,
            user_storage,
            vault_storage,
            mr_storage,
            issue_storage,
            conversation_storage,
            lfs_file_storage,
            note_storage,
        };
        Storage {
            app_service: app_service.into(),
            config: Arc::downgrade(&config),
            issue_service: IssueService::new(base.clone()),
            mr_service: MRService::new(base.clone()),
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

    pub fn mr_storage(&self) -> MrStorage {
        self.app_service.mr_storage.clone()
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

    pub fn mock() -> Self {
        // During test time, we don't need a AppContext,
        // Put config in a leaked static variable thus the weak reference will always be valid.
        static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());

        Storage {
            app_service: AppService::mock(),
            issue_service: IssueService::mock(),
            mr_service: MRService::mock(),
            config: Arc::downgrade(&*CONFIG),
        }
    }
}
