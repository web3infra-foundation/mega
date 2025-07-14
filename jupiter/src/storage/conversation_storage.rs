use std::ops::Deref;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_conversation, reactions};
use common::errors::MegaError;

use crate::model::conv_dto::ConvWithReactions;
use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct ConversationStorage {
    pub base: BaseStorage,
}

impl Deref for ConversationStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ConversationStorage {
    pub async fn add_conversation(
        &self,
        link: &str,
        username: &str,
        comment: Option<String>,
        conv_type: ConvTypeEnum,
    ) -> Result<i64, MegaError> {
        let conversation = mega_conversation::Model::new(link, conv_type, comment, username);
        let conversation = conversation.into_active_model();
        let res = conversation.insert(self.get_connection()).await.unwrap();
        Ok(res.id)
    }

    pub async fn remove_conversation(&self, id: i64) -> Result<(), MegaError> {
        mega_conversation::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn add_reactions(
        &self,
        content: Option<String>,
        subject_id: i64,
        subject_type: &str,
        username: &str,
    ) -> Result<reactions::Model, MegaError> {
        let reactions = reactions::Model::new(content, subject_id, subject_type, username);
        let a_model = reactions.into_active_model();
        let res = a_model.insert(self.get_connection()).await?;
        Ok(res)
    }

    pub async fn get_comments_with_reactions(
        &self,
        link: &str,
    ) -> Result<Vec<ConvWithReactions>, MegaError> {
        let conversations = mega_conversation::Entity::find()
            .filter(mega_conversation::Column::Link.eq(link))
            .find_with_related(reactions::Entity)
            .all(self.get_connection())
            .await?;

        let results = conversations
            .into_iter()
            .map(|(conversation, reactions)| ConvWithReactions {
                conversation,
                reactions,
            })
            .collect();
        Ok(results)
    }

    pub async fn delete_reaction(
        &self,
        pub_reaction_id: &str,
        username: &str,
    ) -> Result<(), MegaError> {
        let _ = reactions::Entity::delete_many()
            .filter(reactions::Column::PublicId.eq(pub_reaction_id))
            .filter(reactions::Column::Username.eq(username))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }
}
