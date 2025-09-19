use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::entity_ext::generate_id;
use callisto::mega_mr_reviewer;
use common::errors::MegaError;
use sea_orm::QueryFilter;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use sea_orm::{ColumnTrait, Set};
use std::ops::Deref;

#[derive(Clone)]
pub struct MrReviewerStorage {
    pub base: BaseStorage,
}

impl Deref for MrReviewerStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl MrReviewerStorage {
    pub fn new_reviewer(&self, mr_link: &str, username: &str) -> mega_mr_reviewer::Model {
        let now = chrono::Utc::now().naive_utc();
        mega_mr_reviewer::Model {
            id: generate_id(),
            mr_link: mr_link.to_string(),
            approved: false,
            username: username.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    pub async fn add_reviewers(
        &self,
        mr_link: &str,
        reviewers: Vec<String>,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            let new_reviewer = self.new_reviewer(mr_link, &reviewer);
            let a_model: mega_mr_reviewer::ActiveModel = new_reviewer.into_active_model();
            a_model.insert(self.get_connection()).await.map_err(|e| {
                tracing::error!("{}", e);
                MegaError::with_message(format!("reviewer {}", reviewer.clone()))
            })?;
        }
        Ok(())
    }

    pub async fn remove_reviewers(
        &self,
        mr_link: &str,
        reviewers: Vec<String>,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            mega_mr_reviewer::Entity::delete_many()
                .filter(mega_mr_reviewer::Column::MrLink.eq(mr_link))
                .filter(mega_mr_reviewer::Column::Username.eq(reviewer.clone()))
                .exec(self.get_connection())
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                    MegaError::with_message(format!("fail to remove reviewer {}", reviewer.clone()))
                })?;
        }
        Ok(())
    }

    pub async fn is_reviewer(&self, mr_link: &str, username: &str) -> Result<bool, MegaError> {
        mega_mr_reviewer::Entity::find()
            .filter(mega_mr_reviewer::Column::MrLink.eq(mr_link))
            .filter(mega_mr_reviewer::Column::Username.eq(username))
            .one(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("Error finding the reviewer: {}", e);
                e
            })?;

        Ok(true)
    }

    pub async fn list_reviewers(
        &self,
        mr_link: &str,
    ) -> Result<Vec<mega_mr_reviewer::Model>, MegaError> {
        let reviewers = mega_mr_reviewer::Entity::find()
            .filter(mega_mr_reviewer::Column::MrLink.eq(mr_link))
            .all(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::with_message(format!("fail to list reviewers for {mr_link}"))
            })?;
        Ok(reviewers)
    }

    pub async fn reviewer_change_state(
        &self,
        mr_link: &str,
        reviewer_username: &str,
        approved: bool,
    ) -> Result<(), MegaError> {
        let mut rev: mega_mr_reviewer::ActiveModel = mega_mr_reviewer::Entity::find()
            .filter(mega_mr_reviewer::Column::MrLink.eq(mr_link))
            .filter(mega_mr_reviewer::Column::Username.eq(reviewer_username))
            .one(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::with_message(format!("fail to find reviewer {}", reviewer_username))
            })?
            .ok_or_else(|| {
                MegaError::with_message(format!("reviewer {} not found", reviewer_username))
            })?
            .into_active_model();

        rev.approved = Set(approved);
        rev.updated_at = Set(chrono::Utc::now().naive_utc());
        rev.update(self.get_connection()).await.map_err(|e| {
            tracing::error!("{}", e);
            MegaError::with_message(format!("fail to update reviewer {}", reviewer_username))
        })?;

        Ok(())
    }
}
