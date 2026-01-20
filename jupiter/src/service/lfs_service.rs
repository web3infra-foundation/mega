use io_orbit::factory::MegaObjectStorageWrapper;

use crate::storage::{
    base_storage::{BaseStorage, StorageConnector},
    lfs_db_storage::LfsDbStorage,
};

#[derive(Clone)]
pub struct LfsService {
    pub lfs_storage: LfsDbStorage,
    pub obj_storage: MegaObjectStorageWrapper,
}

impl LfsService {
    pub fn mock() -> Self {
        let mock = BaseStorage::mock();

        Self {
            lfs_storage: LfsDbStorage { base: mock.clone() },
            obj_storage: MegaObjectStorageWrapper::mock(),
        }
    }
}
