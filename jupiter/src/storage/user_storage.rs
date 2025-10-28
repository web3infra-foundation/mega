use std::ops::Deref;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter,
};
use uuid::Uuid;

use callisto::{access_token, ssh_keys};
use common::{errors::MegaError, utils::generate_id};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct UserStorage {
    pub base: BaseStorage,
}

impl Deref for UserStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl UserStorage {
    pub async fn save_ssh_key(
        &self,
        username: String,
        title: &str,
        ssh_key: &str,
        finger: &str,
    ) -> Result<(), MegaError> {
        let model = ssh_keys::Model {
            id: generate_id(),
            username,
            title: title.to_owned(),
            ssh_key: ssh_key.to_owned(),
            finger: finger.to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
        };
        let a_model = model.into_active_model();
        a_model.insert(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn list_user_ssh(&self, username: String) -> Result<Vec<ssh_keys::Model>, MegaError> {
        let res: Vec<ssh_keys::Model> = ssh_keys::Entity::find()
            .filter(ssh_keys::Column::Username.eq(username))
            .all(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn delete_ssh_key(&self, username: String, id: i64) -> Result<(), MegaError> {
        let res = ssh_keys::Entity::find()
            .filter(ssh_keys::Column::Id.eq(id))
            .filter(ssh_keys::Column::Username.eq(username))
            .one(self.get_connection())
            .await?;
        if let Some(model) = res {
            model.delete(self.get_connection()).await?;
        }
        Ok(())
    }

    pub async fn search_ssh_key_finger(
        &self,
        finger_print: &str,
    ) -> Result<Vec<ssh_keys::Model>, MegaError> {
        let res = ssh_keys::Entity::find()
            .filter(ssh_keys::Column::Finger.eq(finger_print))
            .all(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn generate_token(&self, username: String) -> Result<String, MegaError> {
        let token_str = Uuid::new_v4().to_string();
        let model = access_token::Model {
            id: generate_id(),
            username,
            token: token_str.clone(),
            created_at: chrono::Utc::now().naive_utc(),
        };
        let a_model = model.into_active_model();
        a_model.insert(self.get_connection()).await.unwrap();
        Ok(token_str.to_owned())
    }

    pub async fn delete_token(&self, username: String, id: i64) -> Result<(), MegaError> {
        let res = access_token::Entity::find()
            .filter(access_token::Column::Id.eq(id))
            .filter(access_token::Column::Username.eq(username))
            .one(self.get_connection())
            .await?;
        if let Some(model) = res {
            model.delete(self.get_connection()).await?;
        }
        Ok(())
    }

    pub async fn list_token(
        &self,
        username: String,
    ) -> Result<Vec<access_token::Model>, MegaError> {
        let res = access_token::Entity::find()
            .filter(access_token::Column::Username.eq(username))
            .all(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn check_token(&self, username: &str, token: &str) -> Result<bool, MegaError> {
        let res = access_token::Entity::find()
            .filter(access_token::Column::Username.eq(username))
            .filter(access_token::Column::Token.eq(token))
            .one(self.get_connection())
            .await?;
        match res {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    pub async fn find_user_by_token(&self, token: &str) -> Result<Option<String>, MegaError> {
        let res = access_token::Entity::find()
            .filter(access_token::Column::Token.eq(token))
            .one(self.get_connection())
            .await?;
        match res {
            Some(token_model) => Ok(Some(token_model.username)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use uuid::Uuid;

    #[test]
    fn token_format() {
        let uuid = Uuid::new_v4().to_string();
        println!("{uuid:?}");
    }
}
