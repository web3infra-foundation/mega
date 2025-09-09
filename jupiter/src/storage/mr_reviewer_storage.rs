use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::entity_ext::generate_id;
use callisto::mega_mr_reviewer;
use common::errors::MegaError;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
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
    pub async fn add_reviewers(&self, mr_id: i64, reviewers: Vec<String>) -> Result<(), MegaError> {
        for reviewer in reviewers {
            let new_reviewer = mega_mr_reviewer::Model {
                id: generate_id(),
                mr_id,
                campsite_id: reviewer.clone(),
                approved: false,
            };
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
        mr_id: i64,
        reviewers: Vec<String>,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            mega_mr_reviewer::Entity::delete_many()
                .filter(mega_mr_reviewer::Column::MrId.eq(mr_id))
                .filter(mega_mr_reviewer::Column::CampsiteId.eq(reviewer.clone()))
                .exec(self.get_connection())
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                    MegaError::with_message(format!("fail to remove reviewer {}", reviewer.clone()))
                })?;
        }
        Ok(())
    }

    pub async fn list_reviewers(
        &self,
        mr_id: i64,
    ) -> Result<Vec<mega_mr_reviewer::Model>, MegaError> {
        let reviewers = mega_mr_reviewer::Entity::find()
            .filter(mega_mr_reviewer::Column::MrId.eq(mr_id))
            .all(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::with_message(format!("fail to list reviewers for mr_id {}", mr_id))
            })?;
        Ok(reviewers)
    }

    pub async fn reviewer_change_state(
        &self,
        mr_id: i64,
        reviewer: String,
        approved: bool,
    ) -> Result<(), MegaError> {
        let mut rev: mega_mr_reviewer::ActiveModel = mega_mr_reviewer::Entity::find()
            .filter(mega_mr_reviewer::Column::MrId.eq(mr_id))
            .filter(mega_mr_reviewer::Column::CampsiteId.eq(reviewer.clone()))
            .one(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::with_message(format!("fail to find reviewer {}", reviewer.clone()))
            })?
            .ok_or_else(|| {
                MegaError::with_message(format!("reviewer {} not found", reviewer.clone()))
            })?
            .into_active_model();

        rev.approved = sea_orm::ActiveValue::Set(approved);
        rev.update(self.get_connection()).await.map_err(|e| {
            tracing::error!("{}", e);
            MegaError::with_message(format!("fail to update reviewer {}", reviewer.clone()))
        })?;

        Ok(())
    }
}
