pub mod base_storage;
pub mod git_db_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod mono_storage;
pub mod mr_storage;
pub mod raw_db_storage;
pub mod relay_storage;
pub mod stg_common;
pub mod user_storage;
pub mod vault_storage;

use std::sync::{Arc, LazyLock, Weak};

use common::config::Config;

use crate::lfs_storage::{self, local_storage::LocalStorage, LfsFileStorage};
use crate::storage::init::database_connection;
use crate::storage::{
    git_db_storage::GitDbStorage, issue_storage::IssueStorage, lfs_db_storage::LfsDbStorage,
    mono_storage::MonoStorage, mr_storage::MrStorage, raw_db_storage::RawDbStorage,
    relay_storage::RelayStorage, user_storage::UserStorage, vault_storage::VaultStorage,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct Service {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub raw_db_storage: RawDbStorage,
    pub lfs_db_storage: LfsDbStorage,
    pub relay_storage: RelayStorage,
    pub user_storage: UserStorage,
    pub vault_storage: VaultStorage,
    pub mr_storage: MrStorage,
    pub issue_storage: IssueStorage,
    pub lfs_file_storage: Arc<dyn LfsFileStorage>,
}

impl Service {
    async fn new(config: &Config) -> Self {
        let connection = Arc::new(database_connection(&config.database).await);
        let base = BaseStorage::new(connection.clone());

        Self {
            mono_storage: MonoStorage { base: base.clone() },
            git_db_storage: GitDbStorage { base: base.clone() },
            raw_db_storage: RawDbStorage { base: base.clone() },
            lfs_db_storage: LfsDbStorage { base: base.clone() },
            relay_storage: RelayStorage { base: base.clone() },
            user_storage: UserStorage { base: base.clone() },
            mr_storage: MrStorage { base: base.clone() },
            issue_storage: IssueStorage { base: base.clone() },
            vault_storage: VaultStorage { base: base.clone() },
            lfs_file_storage: lfs_storage::init(config.lfs.clone(), connection.clone()).await,
        }
    }

    fn mock() -> Arc<Self> {
        let mock = BaseStorage::mock();
        Arc::new(Self {
            mono_storage: MonoStorage { base: mock.clone() },
            git_db_storage: GitDbStorage { base: mock.clone() },
            raw_db_storage: RawDbStorage { base: mock.clone() },
            lfs_db_storage: LfsDbStorage { base: mock.clone() },
            relay_storage: RelayStorage { base: mock.clone() },
            user_storage: UserStorage { base: mock.clone() },
            vault_storage: VaultStorage { base: mock.clone() },
            lfs_file_storage: Arc::new(LocalStorage::mock()),
            mr_storage: MrStorage { base: mock.clone() },
            issue_storage: IssueStorage { base: mock.clone() },
        })
    }
}

#[derive(Clone)]
pub struct Storage {
    pub services: Arc<Service>,
    pub config: Weak<Config>,
}

impl Storage {
    pub async fn new(config: Arc<Config>) -> Self {
        Storage {
            services: Service::new(&config).await.into(),
            config: Arc::downgrade(&config),
        }
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.upgrade().expect("Config has been dropped")
    }

    pub fn mono_storage(&self) -> MonoStorage {
        self.services.mono_storage.clone()
    }

    pub fn git_db_storage(&self) -> GitDbStorage {
        self.services.git_db_storage.clone()
    }

    pub fn raw_db_storage(&self) -> RawDbStorage {
        self.services.raw_db_storage.clone()
    }

    pub fn lfs_db_storage(&self) -> LfsDbStorage {
        self.services.lfs_db_storage.clone()
    }

    pub fn relay_storage(&self) -> RelayStorage {
        self.services.relay_storage.clone()
    }

    pub fn user_storage(&self) -> UserStorage {
        self.services.user_storage.clone()
    }

    pub fn vault_storage(&self) -> VaultStorage {
        self.services.vault_storage.clone()
    }

    pub fn mr_storage(&self) -> MrStorage {
        self.services.mr_storage.clone()
    }

    pub fn issue_storage(&self) -> IssueStorage {
        self.services.issue_storage.clone()
    }

    pub fn lfs_file_storage(&self) -> Arc<dyn LfsFileStorage> {
        self.services.lfs_file_storage.clone()
    }

    pub fn mock() -> Self {
        // During test time, we don't need a AppContext,
        // Put config in a leaked static variable thus the weak reference will always be valid.
        static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());

        Storage {
            services: Service::mock(),
            config: Arc::downgrade(&*CONFIG),
        }
    }
}
