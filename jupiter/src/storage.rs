pub mod git_db_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod mono_storage;
pub mod mq_storage;
pub mod mr_storage;
pub mod raw_db_storage;
pub mod relay_storage;
pub mod user_storage;
pub mod vault_storage;

use sea_orm::{sea_query::OnConflict, ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait};

use common::errors::MegaError;

use std::sync::{Arc, LazyLock, Weak};

use common::config::Config;

use crate::lfs_storage::{self, local_storage::LocalStorage, LfsFileStorage};
use crate::storage::init::database_connection;
use crate::storage::{
    git_db_storage::GitDbStorage, issue_storage::IssueStorage, lfs_db_storage::LfsDbStorage,
    mono_storage::MonoStorage, mq_storage::MQStorage, mr_storage::MrStorage,
    raw_db_storage::RawDbStorage, relay_storage::RelayStorage, user_storage::UserStorage,
    vault_storage::VaultStorage,
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
            lfs_file_storage: lfs_storage::init(config.lfs.clone(), lfs_db_storage).await,
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
        // During test time, we don't need a AppContext,
        // Put config in a leaked static variable thus the weak reference will always be valid.
        static CONFIG: LazyLock<Arc<Config>> = LazyLock::new(|| Config::mock().into());

        Storage {
            services: Service::mock(),
            config: Arc::downgrade(&*CONFIG),
        }
    }
}

/// Performs batch saving of models in the database.
///
/// The method takes a vector of models to be saved and performs batch inserts using the given entity type `E`.
/// The models should implement the `ActiveModelTrait` trait, which provides the necessary functionality for saving and inserting the models.
///
/// The method splits the models into smaller chunks, each containing models configured by chunk_size, and inserts them into the database using the `E::insert_many` function.
/// The results of each insertion are collected into a vector of futures.
///
/// Note: Currently, SQLx does not support packets larger than 16MB.
/// # Arguments
///
/// * `save_models` - A vector of models to be saved.
///
/// # Generic Constraints
///
/// * `E` - The entity type that implements the `EntityTrait` trait.
/// * `A` - The model type that implements the `ActiveModelTrait` trait and is convertible from the corresponding model type of `E`.
///
/// # Errors
///
/// Returns a `MegaError` if an error occurs during the batch save operation.
pub async fn batch_save_model<E, A>(
    connection: &impl ConnectionTrait,
    save_models: Vec<A>,
) -> Result<(), MegaError>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let onconflict = OnConflict::new().do_nothing().to_owned();
    batch_save_model_with_conflict(connection, save_models, onconflict).await
}

/// Performs batch saving of models in the database with conflict resolution.
///
/// This function allows saving models in batches while specifying conflict resolution behavior using the `OnConflict` parameter.
/// It is intended for advanced use cases where fine-grained control over conflict handling is required.
///
/// # Arguments
///
/// * `connection` - A reference to the database connection.
/// * `save_models` - A vector of models to be saved.
/// * `onconflict` - Specifies the conflict resolution strategy to be used during insertion.
///
/// # Generic Constraints
///
/// * `E` - The entity type that implements the `EntityTrait` trait.
/// * `A` - The model type that implements the `ActiveModelTrait` trait and is convertible from the corresponding model type of `E`.
///
/// # Errors
///
/// Returns a `MegaError` if an error occurs during the batch save operation.
/// Note: The function ignores `DbErr::RecordNotInserted` errors, which may lead to silent failures.
/// Use this function with caution and ensure that the `OnConflict` parameter is configured correctly to avoid unintended consequences.
pub async fn batch_save_model_with_conflict<E, A>(
    connection: &impl ConnectionTrait,
    save_models: Vec<A>,
    onconflict: OnConflict,
) -> Result<(), MegaError>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    // notice that sqlx not support packets larger than 16MB now
    let futures = save_models.chunks(1000).map(|chunk| {
        let insert = E::insert_many(chunk.iter().cloned()).on_conflict(onconflict.clone());
        let conn = connection;
        async move {
            match insert.exec(conn).await {
                Ok(_) => Ok(()),
                Err(DbErr::RecordNotInserted) => {
                    // ignore not inserted err
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    });
    futures::future::try_join_all(futures).await?;
    Ok(())
}
