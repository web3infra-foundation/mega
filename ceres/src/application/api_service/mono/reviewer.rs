use common::errors::MegaError;

use super::service::MonoApiService;
use crate::model::change_list::{ReviewerInfo, ReviewersResponse};

impl MonoApiService {
    pub async fn add_reviewers(&self, link: &str, reviewers: Vec<String>) -> Result<(), MegaError> {
        self.storage
            .reviewer_storage()
            .add_reviewers(link, reviewers)
            .await
    }

    pub async fn remove_reviewers(
        &self,
        link: &str,
        reviewers: &[String],
    ) -> Result<(), MegaError> {
        self.storage
            .reviewer_storage()
            .remove_reviewers(link, reviewers)
            .await
    }

    pub async fn list_reviewers(&self, link: &str) -> Result<ReviewersResponse, MegaError> {
        let reviewers = self
            .storage
            .reviewer_storage()
            .list_reviewers(link)
            .await?
            .into_iter()
            .map(|r| ReviewerInfo {
                username: r.username,
                approved: r.approved,
                system_required: r.system_required,
            })
            .collect();
        Ok(ReviewersResponse { result: reviewers })
    }

    pub async fn reviewer_change_state(
        &self,
        link: &str,
        username: &str,
        approved: bool,
    ) -> Result<(), MegaError> {
        self.storage
            .reviewer_storage()
            .reviewer_change_state(link, username, approved)
            .await
    }

    pub async fn is_reviewer(&self, link: &str, username: &str) -> Result<bool, MegaError> {
        self.storage
            .reviewer_storage()
            .is_reviewer(link, username)
            .await
    }
}
