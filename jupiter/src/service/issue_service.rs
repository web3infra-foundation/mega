use crate::{
    model::issue_dto::IssueDetails,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        conversation_storage::ConversationStorage,
        issue_storage::IssueStorage,
        mr_storage::MrStorage,
    },
};

use callisto::{mega_issue, mega_mr};
use common::errors::MegaError;

#[derive(Clone)]
pub struct IssueService {
    pub issue_storage: IssueStorage,
    pub mr_storage: MrStorage,
    pub conversation_storage: ConversationStorage,
}

impl IssueService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            issue_storage: IssueStorage {
                base: base_storage.clone(),
            },
            conversation_storage: ConversationStorage {
                base: base_storage.clone(),
            },
            mr_storage: MrStorage {
                base: base_storage.clone(),
            },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
            mr_storage: MrStorage { base: mock.clone() },
        }
    }

    pub async fn get_issue_details(
        &self,
        link: &str,
        username: String,
    ) -> Result<IssueDetails, MegaError> {
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
            username,
        };
        Ok(res)
    }

    pub async fn get_suggestions(
        &self,
        query: &str,
    ) -> Result<(Vec<mega_issue::Model>, Vec<mega_mr::Model>), MegaError> {
        let issues = self
            .issue_storage
            .get_issue_suggestions_by_query(query)
            .await?;
        let mrs = self.mr_storage.get_mr_suggestions_by_query(query).await?;
        Ok((issues, mrs))
    }
}
