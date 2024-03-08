use std::sync::Arc;

use common::enums::DataSource;
use storage::driver::database::{self, mysql_storage::MysqlStorage, storage::ObjectStorage};

use crate::storage::{
    git_storage::GitStorage, init::database_connection, mega_storage::MegaStorage,
};

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub storage: Arc<dyn ObjectStorage> ,
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
}

impl Service {
    async fn new() -> Service {
        Service {
            mega_storage: Arc::new(MegaStorage::new(database_connection().await).await),
            git_storage: Arc::new(GitStorage::new().await),
        }
    }

    async fn shared() -> Arc<Self> {
        Arc::new(Self::new().await)
    }

    fn mock() -> Arc<Self> {
        Arc::new(Self {
            mega_storage: Arc::new(MegaStorage::mock()),
            git_storage: Arc::new(GitStorage::mock())
        })
    }
}
