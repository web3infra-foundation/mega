use std::sync::Arc;

use callisto::mq_storage;
use sea_orm::{DatabaseConnection, EntityTrait, InsertResult, IntoActiveModel, Set};


#[derive(Clone)]
pub struct MQStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl MQStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        MQStorage { connection }
    }

    pub fn mock() -> Self {
        MQStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }


}
