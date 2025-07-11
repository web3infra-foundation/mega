use std::collections::HashMap;
use std::ops::Deref;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_conversation, reactions};
use common::errors::MegaError;

use crate::model::issue_dto::ConvWithReactions;
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
            .all(self.get_connection())
            .await?;

        let conv_ids = conversations.iter().map(|c| c.id).collect::<Vec<_>>();

        let reactions = reactions::Entity::find()
            .filter(reactions::Column::SubjectId.is_in(conv_ids.clone()))
            .all(self.get_connection())
            .await?;

        let mut conv_map = HashMap::new();
        for conversation in conversations {
            let related = reactions
                .iter()
                .filter(|r| r.subject_id == conversation.id)
                .cloned()
                .collect();
            conv_map.insert(
                conversation.id,
                ConvWithReactions {
                    conversation,
                    reactions: related,
                },
            );
        }

        Ok(conv_map.into_values().collect())
    }
}
