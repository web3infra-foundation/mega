use std::sync::Arc;

use common::config::Config;

use crate::{
    lfs_storage::{self, local_storage::LocalStorage, LfsFileStorage},
    storage::{
        git_db_storage::GitDbStorage, init::database_connection, issue_storage::IssueStorage,
        lfs_db_storage::LfsDbStorage, mono_storage::MonoStorage, mq_storage::MQStorage,
        mr_storage::MrStorage, raw_db_storage::RawDbStorage, relay_storage::RelayStorage,
        user_storage::UserStorage, vault_storage::VaultStorage,
    },
};

#[derive(Clone)]
pub struct Service {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub raw_db_storage: RawDbStorage,
    pub lfs_db_storage: LfsDbStorage,
    pub relay_storage: RelayStorage,
    pub mq_storage: MQStorage,
    pub user_storage: UserStorage,
    pub vault_storage: VaultStorage,
    pub mr_storage: MrStorage,
    pub issue_storage: IssueStorage,
    pub lfs_file_storage: Arc<dyn LfsFileStorage>,
}

impl Service {
    async fn new(config: &Config) -> Self {
        let connection = Arc::new(database_connection(&config.database).await);
        let lfs_db_storage = LfsDbStorage::new(connection.clone()).await;

        Self {
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
            lfs_file_storage: lfs_storage::init(config.lfs.clone(), lfs_db_storage.clone()).await,
        }
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mono_storage: MonoStorage::mock(),
            git_db_storage: GitDbStorage::mock(),
            raw_db_storage: RawDbStorage::mock(),
            lfs_db_storage: LfsDbStorage::mock(),
            relay_storage: RelayStorage::mock(),
            mq_storage: MQStorage::mock(),
            user_storage: UserStorage::mock(),
            vault_storage: VaultStorage::mock(),
            lfs_file_storage: Arc::new(LocalStorage::mock()),
            mr_storage: MrStorage::mock(),
            issue_storage: IssueStorage::mock(),
        })
    }
}

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub config: Arc<Config>,
}

impl Context {
    pub async fn new(config: Arc<Config>) -> Self {
        Context {
            services: Service::new(&config).await.into(),
            config,
        }
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

    pub fn mq_storage(&self) -> MQStorage {
        self.services.mq_storage.clone()
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
        Context {
            services: Service::mock(),
            config: Arc::new(Config::mock()),
        }
    }
}
