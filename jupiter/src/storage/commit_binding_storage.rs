use std::ops::Deref;

use callisto::commit_auths::{ActiveModel, Column::CommitSha, Entity};
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use uuid::Uuid;

use crate::storage::base_storage::{BaseStorage, StorageConnector};
#[derive(Clone)]
pub struct CommitBindingStorage {
    pub base: BaseStorage,
}
impl Deref for CommitBindingStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl CommitBindingStorage {
    pub fn new(base: BaseStorage) -> Self {
        Self { base }
    }

    /// Save or update a commit binding
    pub async fn upsert_binding(
        &self,
        sha: &str,
        matched_username: Option<String>,
        is_anonymous: bool,
    ) -> Result<(), MegaError> {
        let now = chrono::Utc::now().naive_utc();
        let model = ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            commit_sha: ActiveValue::Set(sha.to_string()),
            matched_username: ActiveValue::Set(matched_username.clone()),
            is_anonymous: ActiveValue::Set(is_anonymous),
            matched_at: ActiveValue::Set(Some(now)),
            created_at: ActiveValue::Set(now),
        };

        // Try insert, if conflict update
        let conn = self.get_connection();
        // Simple upsert: try find then insert/update
        let existing = Entity::find()
            .filter(CommitSha.eq(sha.to_string()))
            .one(conn)
            .await?;

        if let Some(e) = existing {
            let mut am = e.into_active_model();
            am.matched_username = ActiveValue::Set(matched_username.clone());
            am.is_anonymous = ActiveValue::Set(is_anonymous);
            am.matched_at = ActiveValue::Set(Some(now));
            am.update(conn).await?;
        } else {
            model.insert(conn).await?;
        }

        Ok(())
    }

    pub async fn find_by_sha(
        &self,
        sha: &str,
    ) -> Result<Option<callisto::commit_auths::Model>, MegaError> {
        let conn = self.get_connection();
        let res = Entity::find()
            .filter(CommitSha.eq(sha.to_string()))
            .one(conn)
            .await?;
        Ok(res)
    }
}
