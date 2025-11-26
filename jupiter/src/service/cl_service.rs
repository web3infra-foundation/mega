use crate::{
    model::cl_dto::CLDetails,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        cl_storage::ClStorage,
        conversation_storage::ConversationStorage,
    },
};

use common::errors::MegaError;

#[derive(Clone)]
pub struct CLService {
    pub cl_storage: ClStorage,
    pub conversation_storage: ConversationStorage,
}

impl CLService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
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
            cl_storage: ClStorage { base: mock.clone() },
            conversation_storage: ConversationStorage { base: mock.clone() },
        }
    }

    pub async fn get_cl_details(
        &self,
        link: &str,
        username: String,
    ) -> Result<CLDetails, MegaError> {
        let (cl, labels) = self
            .cl_storage
            .get_cl_labels(link)
            .await?
            .ok_or_else(|| MegaError::Other("CL not found".to_string()))?;

        let conversations = self
            .conversation_storage
            .get_comments_with_reactions(link)
            .await?;

        let (_, assignees) = self
            .cl_storage
            .get_cl_assignees(link)
            .await?
            .unwrap_or((cl.clone(), vec![]));

        let res = CLDetails {
            cl,
            labels,
            conversations,
            assignees,
            username,
        };
        Ok(res)
    }
}
