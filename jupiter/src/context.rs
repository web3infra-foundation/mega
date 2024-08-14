use std::sync::Arc;

use common::config::Config;

use crate::storage::{
    git_db_storage::GitDbStorage, init::database_connection, lfs_storage::LfsStorage,
    mono_storage::MonoStorage, mq_storage::MQStorage, ztm_storage::ZTMStorage,
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
    pub mono_storage: Arc<MonoStorage>,
    pub git_db_storage: Arc<GitDbStorage>,
    pub lfs_storage: Arc<LfsStorage>,
    pub ztm_storage: Arc<ZTMStorage>,
    pub mq_storage: Arc<MQStorage>,
}

impl Service {
    async fn new(config: &Config) -> Service {
        let connection = Arc::new(database_connection(&config.database).await);
        Service {
            mono_storage: Arc::new(
                MonoStorage::new(connection.clone(), config.storage.clone()).await,
            ),
            git_db_storage: Arc::new(
                GitDbStorage::new(connection.clone(), config.storage.clone()).await,
            ),
            lfs_storage: Arc::new(LfsStorage::new(connection.clone()).await),
            ztm_storage: Arc::new(ZTMStorage::new(connection.clone()).await),
            mq_storage: Arc::new(MQStorage::new(connection.clone()).await),
        }
    }

    async fn shared(config: &Config) -> Arc<Self> {
        Arc::new(Self::new(config).await)
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mono_storage: Arc::new(MonoStorage::mock()),
            git_db_storage: Arc::new(GitDbStorage::mock()),
            lfs_storage: Arc::new(LfsStorage::mock()),
            ztm_storage: Arc::new(ZTMStorage::mock()),
            mq_storage: Arc::new(MQStorage::mock()),
        })
    }
}
