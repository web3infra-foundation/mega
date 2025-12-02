use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::entity_ext::generate_id;
use callisto::mega_cl_reviewer;
use common::errors::MegaError;
use sea_orm::QueryFilter;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use sea_orm::{ColumnTrait, Set};
use std::ops::Deref;

#[derive(Clone)]
pub struct ClReviewerStorage {
    pub base: BaseStorage,
}

impl Deref for ClReviewerStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ClReviewerStorage {
    pub fn new_reviewer(&self, cl_link: &str, username: &str) -> mega_cl_reviewer::Model {
        let now = chrono::Utc::now().naive_utc();
        mega_cl_reviewer::Model {
            id: generate_id(),
            cl_link: cl_link.to_string(),
            approved: false,
            username: username.to_string(),
            created_at: now,
            updated_at: now,
            system_required: false,
        }
    }

    pub async fn add_reviewers(
        &self,
        cl_link: &str,
        reviewers: Vec<String>,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            let new_reviewer = self.new_reviewer(cl_link, &reviewer);
            let a_model: mega_cl_reviewer::ActiveModel = new_reviewer.into_active_model();
            a_model.insert(self.get_connection()).await.map_err(|e| {
                tracing::error!("{}", e);
                MegaError::Other(format!("reviewer {}", reviewer.clone()))
            })?;
        }
        Ok(())
    }

    pub async fn update_system_required_reviewers(
        &self,
        cl_link: &str,
        reviewers: &Vec<String>,
        system_required: bool,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            let mut rev: mega_cl_reviewer::ActiveModel = mega_cl_reviewer::Entity::find()
                .filter(mega_cl_reviewer::Column::ClLink.eq(cl_link))
                .filter(mega_cl_reviewer::Column::Username.eq(reviewer))
                .one(self.get_connection())
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                    MegaError::Other(format!("fail to find reviewer {}", reviewer))
                })?
                .ok_or_else(|| MegaError::Other(format!("reviewer {} not found", reviewer)))?
                .into_active_model();

            rev.system_required = Set(system_required);
            rev.updated_at = Set(chrono::Utc::now().naive_utc());
            rev.update(self.get_connection()).await.map_err(|e| {
                tracing::error!("{}", e);
                MegaError::Other(format!(
                    "fail to update system required for reviewer {}",
                    reviewer
                ))
            })?;
        }
        Ok(())
    }

    pub async fn remove_reviewers(
        &self,
        cl_link: &str,
        reviewers: &Vec<String>,
    ) -> Result<(), MegaError> {
        for reviewer in reviewers {
            mega_cl_reviewer::Entity::delete_many()
                .filter(mega_cl_reviewer::Column::ClLink.eq(cl_link))
                .filter(mega_cl_reviewer::Column::Username.eq(reviewer.clone()))
                .filter(mega_cl_reviewer::Column::SystemRequired.eq(false))
                .exec(self.get_connection())
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                    MegaError::Other(format!("fail to remove reviewer {}", reviewer.clone()))
                })?;
        }
        Ok(())
    }

    pub async fn is_reviewer(&self, cl_link: &str, username: &str) -> Result<bool, MegaError> {
        let is_reviewer = mega_cl_reviewer::Entity::find()
            .filter(mega_cl_reviewer::Column::ClLink.eq(cl_link))
            .filter(mega_cl_reviewer::Column::Username.eq(username))
            .one(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("Error finding the reviewer: {}", e);
                e
            })?
            .is_some();

        Ok(is_reviewer)
    }

    pub async fn list_reviewers(
        &self,
        cl_link: &str,
    ) -> Result<Vec<mega_cl_reviewer::Model>, MegaError> {
        let reviewers = mega_cl_reviewer::Entity::find()
            .filter(mega_cl_reviewer::Column::ClLink.eq(cl_link))
            .all(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::Other(format!("fail to list reviewers for {cl_link}"))
            })?;
        Ok(reviewers)
    }

    pub async fn reviewer_change_state(
        &self,
        cl_link: &str,
        reviewer_username: &str,
        approved: bool,
    ) -> Result<(), MegaError> {
        let mut rev: mega_cl_reviewer::ActiveModel = mega_cl_reviewer::Entity::find()
            .filter(mega_cl_reviewer::Column::ClLink.eq(cl_link))
            .filter(mega_cl_reviewer::Column::Username.eq(reviewer_username))
            .one(self.get_connection())
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                MegaError::Other(format!("fail to find reviewer {}", reviewer_username))
            })?
            .ok_or_else(|| MegaError::Other(format!("reviewer {} not found", reviewer_username)))?
            .into_active_model();

        rev.approved = Set(approved);
        rev.updated_at = Set(chrono::Utc::now().naive_utc());
        rev.update(self.get_connection()).await.map_err(|e| {
            tracing::error!("{}", e);
            MegaError::Other(format!("fail to update reviewer {}", reviewer_username))
        })?;

        Ok(())
    }
}
