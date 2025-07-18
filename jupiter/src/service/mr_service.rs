use crate::{
    model::mr_dto::MRDetails,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        conversation_storage::ConversationStorage,
        mr_storage::MrStorage,
    },
};

use common::errors::MegaError;

#[derive(Clone)]
pub struct MRService {
    pub mr_storage: MrStorage,
    pub conversation_storage: ConversationStorage,
}

impl MRService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
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
            mr_storage: MrStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
        }
    }

    pub async fn get_mr_details(
        &self,
        link: &str,
        username: String,
    ) -> Result<MRDetails, MegaError> {
        let (mr, labels) = self
            .mr_storage
            .get_mr_labels(link)
            .await?
            .ok_or_else(|| MegaError::with_message("MR not found"))?;

        let conversations = self
            .conversation_storage
            .get_comments_with_reactions(link)
            .await?;

        let (_, assignees) = self
            .mr_storage
            .get_mr_assignees(link)
            .await?
            .unwrap_or((mr.clone(), vec![]));

        let res = MRDetails {
            mr,
            labels,
            conversations,
            assignees,
            username,
        };
        Ok(res)
    }
}
