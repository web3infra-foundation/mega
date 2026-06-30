use common::errors::MegaError;

use super::service::MonoApiService;
use crate::model::{
    change_list::MergeStatus,
    conversation::{ConvType, ReferenceType},
};

impl MonoApiService {
    pub async fn add_conversation(
        &self,
        link: &str,
        username: &str,
        comment: Option<String>,
        conv_type: ConvType,
    ) -> Result<i64, MegaError> {
        self.storage
            .conversation_storage()
            .add_conversation(link, username, comment, conv_type.into())
            .await
    }

    pub async fn add_issue_mention_reference(
        &self,
        source_link: &str,
        ref_link: &str,
        username: &str,
    ) -> Result<(), MegaError> {
        self.storage
            .issue_storage()
            .add_reference(source_link, ref_link, ReferenceType::Mention.into())
            .await?;
        self.add_conversation(
            ref_link,
            username,
            Some(format!("{username} mentioned this on")),
            ConvType::Mention,
        )
        .await?;
        Ok(())
    }

    pub async fn cl_merge_status(&self, link: &str) -> Result<MergeStatus, MegaError> {
        let model = self
            .storage
            .cl_storage()
            .get_cl(link)
            .await?
            .ok_or_else(|| MegaError::NotFound(format!("CL {link} not found")))?;
        Ok(model.status.into())
    }

    pub async fn add_comment_reaction(
        &self,
        content: Option<String>,
        comment_id: i64,
        comment_type: &str,
        username: &str,
    ) -> Result<(), MegaError> {
        self.storage
            .conversation_storage()
            .add_reactions(content, comment_id, comment_type, username)
            .await?;
        Ok(())
    }

    pub async fn delete_comment_reaction(
        &self,
        reaction_id: &str,
        username: &str,
    ) -> Result<(), MegaError> {
        self.storage
            .conversation_storage()
            .delete_reaction(reaction_id, username)
            .await
    }

    pub async fn remove_conversation(&self, comment_id: i64) -> Result<(), MegaError> {
        self.storage
            .conversation_storage()
            .remove_conversation(comment_id)
            .await
    }

    pub async fn update_comment(
        &self,
        comment_id: i64,
        content: Option<String>,
    ) -> Result<(), MegaError> {
        self.storage
            .conversation_storage()
            .update_comment(comment_id, content)
            .await
    }

    pub async fn change_review_state(
        &self,
        link: &str,
        conversation_id: &i64,
        resolved: bool,
    ) -> Result<(), MegaError> {
        self.storage
            .conversation_storage()
            .change_review_state(link, conversation_id, resolved)
            .await
    }
}
