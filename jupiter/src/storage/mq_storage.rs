use std::sync::Arc;

use callisto::mq_storage;
use sea_orm::DatabaseConnection;

use super::batch_save_model;


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

    pub async fn save_messages(&self, msgs: Vec<mq_storage::Model>) {
        let msgs: Vec<mq_storage::ActiveModel> = msgs.into_iter().map(|m| m.into()).collect();
        batch_save_model(self.get_connection(), msgs).await.unwrap();
    }

    pub async fn get_latest_messages() -> Vec<mq_storage::Model> {
        todo!()
    }
}
