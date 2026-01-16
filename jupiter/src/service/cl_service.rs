use common::errors::MegaError;

use crate::{
    model::cl_dto::CLDetails,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        cl_storage::ClStorage,
        conversation_storage::ConversationStorage,
    },
};

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

    /// Create a new Draft CL
    ///
    /// # Arguments
    /// * `path` - Repository path
    /// * `link` - CL link
    /// * `title` - CL title
    /// * `from_hash` - Base commit hash
    /// * `username` - User creating the CL
    ///
    /// # Returns
    /// Returns the created CL link on success
    pub async fn create_draft_cl(
        &self,
        path: &str,
        link: &str,
        title: &str,
        from_hash: &str,
        username: &str,
    ) -> Result<String, MegaError> {
        self.cl_storage
            .new_cl_draft(path, link, title, from_hash, username)
            .await
    }
}
