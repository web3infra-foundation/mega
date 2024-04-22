use std::sync::Arc;

use common::enums::DataSource;

use crate::storage::{
    git_db_storage::GitDbStorage, init::database_connection, lfs_storage::LfsStorage,
    mega_storage::MegaStorage,
};

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
}

impl Context {
    pub async fn new(_: &DataSource) -> Self {
        Context {
            services: Service::shared().await,
        }
    }
    pub fn mock() -> Self {
        Context {
            services: Service::mock(),
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
    async fn new() -> Service {
        let connection = Arc::new(database_connection().await);
        Service {
            mega_storage: Arc::new(MegaStorage::new(connection.clone()).await),
            git_db_storage: Arc::new(GitDbStorage::new(connection.clone()).await),
            lfs_storage: Arc::new(LfsStorage::new(connection.clone()).await),
        }
    }

    async fn shared() -> Arc<Self> {
        Arc::new(Self::new().await)
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mega_storage: Arc::new(MegaStorage::mock()),
            git_db_storage: Arc::new(GitDbStorage::mock()),
            lfs_storage: Arc::new(LfsStorage::mock()),
        })
    }
}
