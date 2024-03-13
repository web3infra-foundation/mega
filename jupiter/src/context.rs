use std::sync::Arc;

use common::enums::DataSource;
use storage::driver::database::{self, mysql_storage::MysqlStorage, storage::ObjectStorage};

use crate::storage::{
    git_storage::GitStorage, init::database_connection, lfs_storage::LfsStorage,
    mega_storage::MegaStorage,
};

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub storage: Arc<dyn ObjectStorage>,
}

impl Context {
    pub async fn new(data_source: &DataSource) -> Self {
        Context {
            services: Service::shared().await,
            storage: database::init(data_source).await,
        }
    }
    pub fn mock() -> Self {
        Context {
            services: Service::mock(),
            storage: Arc::new(MysqlStorage::default()),
        }
    }
}

#[derive(Clone)]
pub struct Service {
    pub mega_storage: Arc<MegaStorage>,
    pub git_storage: Arc<GitStorage>,
    pub lfs_storage: Arc<LfsStorage>,
}

impl Service {
    async fn new() -> Service {
        let connection = Arc::new(database_connection().await);
        Service {
            mega_storage: Arc::new(MegaStorage::new(connection.clone()).await),
            git_storage: Arc::new(GitStorage::new().await),
            lfs_storage: Arc::new(LfsStorage::new(connection.clone()).await),
        }
    }

    async fn shared() -> Arc<Self> {
        Arc::new(Self::new().await)
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mega_storage: Arc::new(MegaStorage::mock()),
            git_storage: Arc::new(GitStorage::mock()),
            lfs_storage: Arc::new(LfsStorage::mock()),
        })
    }
}
