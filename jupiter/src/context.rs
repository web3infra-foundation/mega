use std::sync::Arc;

use common::config::Config;

use crate::{
    lfs_storage::{self, local_storage::LocalStorage, LfsFileStorage},
    storage::{
        git_db_storage::GitDbStorage, init::database_connection, issue_storage::IssueStorage,
        lfs_db_storage::LfsDbStorage, mono_storage::MonoStorage, mq_storage::MQStorage,
        mr_storage::MrStorage, raw_db_storage::RawDbStorage, user_storage::UserStorage,
        ztm_storage::ZTMStorage,
    },
};

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub config: Arc<Config>,
}

impl Context {
    pub async fn new(config: Arc<Config>) -> Self {
        Context {
            services: Service::shared(&config).await,
            config,
        }
    }

    pub fn user_stg(&self) -> UserStorage {
        self.services.user_storage()
    }

    pub fn issue_stg(&self) -> IssueStorage {
        self.services.issue_storage()
    }

    pub fn mr_stg(&self) -> MrStorage {
        self.services.mr_storage()
    }

    pub fn lfs_stg(&self) -> LfsDbStorage {
        self.services.lfs_db_storage()
    }

    pub fn lfs_file_stg(&self) -> Arc<dyn LfsFileStorage> {
        self.services.lfs_file_storage()
    }

    pub fn mock() -> Self {
        Context {
            services: Service::mock(),
            config: Arc::new(Config::default()),
        }
    }
}

#[derive(Clone)]
pub struct Service {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub raw_db_storage: RawDbStorage,
    lfs_db_storage: LfsDbStorage,
    pub ztm_storage: ZTMStorage,
    pub mq_storage: MQStorage,
    user_storage: UserStorage,
    mr_storage: MrStorage,
    issue_storage: IssueStorage,
    lfs_file_storage: Arc<dyn LfsFileStorage>,
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
            ztm_storage: ZTMStorage::new(connection.clone()).await,
            mq_storage: MQStorage::new(connection.clone()).await,
            user_storage: UserStorage::new(connection.clone()).await,
            mr_storage: MrStorage::new(connection.clone()).await,
            issue_storage: IssueStorage::new(connection.clone()).await,
            lfs_file_storage: lfs_storage::init(config.lfs.clone(), lfs_db_storage.clone()).await,
        }
    }

    async fn shared(config: &Config) -> Arc<Self> {
        Arc::new(Self::new(config).await)
    }

    fn issue_storage(&self) -> IssueStorage {
        self.issue_storage.clone()
    }

    fn mr_storage(&self) -> MrStorage {
        self.mr_storage.clone()
    }

    fn user_storage(&self) -> UserStorage {
        self.user_storage.clone()
    }

    fn lfs_db_storage(&self) -> LfsDbStorage {
        self.lfs_db_storage.clone()
    }

    fn lfs_file_storage(&self) -> Arc<dyn LfsFileStorage> {
        self.lfs_file_storage.clone()
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mono_storage: MonoStorage::mock(),
            git_db_storage: GitDbStorage::mock(),
            raw_db_storage: RawDbStorage::mock(),
            lfs_db_storage: LfsDbStorage::mock(),
            ztm_storage: ZTMStorage::mock(),
            mq_storage: MQStorage::mock(),
            user_storage: UserStorage::mock(),
            lfs_file_storage: Arc::new(LocalStorage::mock()),
            mr_storage: MrStorage::mock(),
            issue_storage: IssueStorage::mock(),
        })
    }
}
