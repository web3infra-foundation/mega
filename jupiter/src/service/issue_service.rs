use callisto::{mega_cl, mega_issue};
use common::errors::MegaError;

use crate::{
    model::issue_dto::IssueDetails,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        cl_storage::ClStorage,
        conversation_storage::ConversationStorage,
        issue_storage::IssueStorage,
    },
};

#[derive(Clone)]
pub struct IssueService {
    pub issue_storage: IssueStorage,
    pub cl_storage: ClStorage,
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
            cl_storage: ClStorage {
                base: base_storage.clone(),
            },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            issue_storage: IssueStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
            cl_storage: ClStorage { base: mock.clone() },
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
            .ok_or_else(|| MegaError::Other("Issue not found".to_string()))?;

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
    ) -> Result<(Vec<mega_issue::Model>, Vec<mega_cl::Model>), MegaError> {
        let issues = self
            .issue_storage
            .get_issue_suggestions_by_query(query)
            .await?;
        let cls = self.cl_storage.get_cl_suggestions_by_query(query).await?;
        Ok((issues, cls))
    }
}
