use std::sync::Arc;

use common::config::Config;

use crate::storage::{
    git_db_storage::GitDbStorage, init::database_connection, lfs_storage::LfsStorage,
    mega_storage::MegaStorage,
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
    pub mega_storage: Arc<MegaStorage>,
    pub git_db_storage: Arc<GitDbStorage>,
    pub lfs_storage: Arc<LfsStorage>,
}

impl Service {
    async fn new(config: &Config) -> Service {
        let connection = Arc::new(database_connection(&config.database).await);
        Service {
            mega_storage: Arc::new(
                MegaStorage::new(connection.clone(), config.storage.clone()).await,
            ),
            git_db_storage: Arc::new(
                GitDbStorage::new(connection.clone(), config.storage.clone()).await,
            ),
            lfs_storage: Arc::new(LfsStorage::new(connection.clone()).await),
        }
    }

    async fn shared(config: &Config) -> Arc<Self> {
        Arc::new(Self::new(config).await)
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mega_storage: Arc::new(MegaStorage::mock()),
            git_db_storage: Arc::new(GitDbStorage::mock()),
            lfs_storage: Arc::new(LfsStorage::mock()),
        })
    }
}
