use crate::{
    model::issue_dto::IssueDetails,
    service::IssueService,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        conversation_storage::ConversationStorage,
        issue_storage::IssueStorage,
    },
};

use common::errors::MegaError;

impl IssueService {
    pub fn new(issue_storage: IssueStorage, conversation_storage: ConversationStorage) -> Self {
        Self {
            issue_storage,
            conversation_storage,
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
        }
    }

    pub async fn get_issue_details(&self, link: &str) -> Result<IssueDetails, MegaError> {
        let (issue, labels) = self
            .issue_storage
            .get_issue_labels(link)
            .await?
            .ok_or_else(|| MegaError::with_message("Issue not found"))?;

        let conversations = self
            .conversation_storage
            .get_comments_with_reactions(link)
            .await?;

        let (_, assignees) = self
            .issue_storage
            .get_issue_assignees(link)
            .await?
            .unwrap_or((issue.clone(), vec![]));

        let res = IssueDetails {
            issue,
            labels,
            conversations,
            assignees,
        };
        Ok(res)
    }
}
