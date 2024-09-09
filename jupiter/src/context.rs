use std::{env, path::PathBuf, sync::Arc};

use common::config::Config;

use crate::{
    lfs_storage::{local_storage::LocalStorage, LfsStorage},
    storage::{
        git_db_storage::GitDbStorage, init::database_connection, lfs_db_storage::LfsDbStorage,
        mono_storage::MonoStorage, mq_storage::MQStorage, raw_db_storage::RawDbStorage,
        user_storage::UserStorage, ztm_storage::ZTMStorage,
    },
};

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub config: Config,
}

impl Context {
    pub async fn new(config: Config) -> Self {
        Context {
            services: Service::shared(&config).await,
            config,
        }
    }
    pub fn mock() -> Self {
        Context {
            services: Service::mock(),
            config: Config::default(),
        }
    }
}

#[derive(Clone)]
pub struct Service {
    pub mono_storage: MonoStorage,
    pub git_db_storage: GitDbStorage,
    pub raw_db_storage: RawDbStorage,
    pub lfs_db_storage: LfsDbStorage,
    pub ztm_storage: ZTMStorage,
    pub mq_storage: MQStorage,
    pub user_storage: UserStorage,
    pub lfs_storage: Arc<dyn LfsStorage>,
}

impl Service {
    async fn new(config: &Config) -> Service {
        let connection = Arc::new(database_connection(&config.database).await);
        Service {
            mono_storage: MonoStorage::new(connection.clone()).await,
            git_db_storage:GitDbStorage::new(connection.clone()).await,
            raw_db_storage: RawDbStorage::new(connection.clone()).await,
            lfs_db_storage: LfsDbStorage::new(connection.clone()).await,
            ztm_storage: ZTMStorage::new(connection.clone()).await,
            mq_storage: MQStorage::new(connection.clone()).await,
            user_storage: UserStorage::new(connection.clone()).await,
            lfs_storage: Arc::new(LocalStorage::init(config.lfs.lfs_obj_local_path.clone())),
        }
    }

    async fn shared(config: &Config) -> Arc<Self> {
        Arc::new(Self::new(config).await)
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
            lfs_storage: Arc::new(LocalStorage::init(
                PathBuf::from(env::current_dir().unwrap().parent().unwrap()).join("tests"),
            )),
        })
    }
}
