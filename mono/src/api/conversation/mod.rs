use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use callisto::{mega_conversation, reactions, sea_orm_active_enums::ConvTypeEnum};

pub mod conv_router;

#[derive(Serialize, ToSchema)]
pub struct ConversationItem {
    pub id: i64,
    pub username: String,
    pub conv_type: ConvType,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub grouped_reactions: Vec<ReactionItem>,
}

impl ConversationItem {
    pub fn from_model(
        conversation: mega_conversation::Model,
        reactions: Vec<reactions::Model>,
        viewer: &str,
    ) -> Self {
        let mut item = Self {
            id: conversation.id,
            username: conversation.username,
            conv_type: conversation.conv_type.into(),
            comment: conversation.comment,
            created_at: conversation.created_at.and_utc().timestamp(),
            updated_at: conversation.updated_at.and_utc().timestamp(),
            grouped_reactions: vec![],
        };
        item.grouped_emoji(viewer, reactions);
        item
    }

    pub fn grouped_emoji(&mut self, username: &str, reactions: Vec<reactions::Model>) {
        let mut reactions_map: HashMap<String, ReactionItem> = HashMap::new();
        let username = username.to_owned();
        for r in &reactions {
            if let Some(emoji) = &r.content {
                let entry = reactions_map
                    .entry(emoji.clone())
                    .or_insert_with(|| ReactionItem {
                        emoji: emoji.clone(),
                        reactions_count: 0,
                        viewer_reaction_id: String::new(),
                        tooltip: Vec::new(),
                        custom_content: String::new(),
                    });
                entry.reactions_count += 1;
                if r.username == username {
                    entry.viewer_reaction_id = r.public_id.clone();
                }
                entry.tooltip.push(r.username.clone());
            }
        }

        self.grouped_reactions = reactions_map.into_values().collect();
    }
}

#[derive(Serialize, Default, ToSchema)]
pub struct ReactionItem {
    pub viewer_reaction_id: String,
    pub emoji: String,
    pub tooltip: Vec<String>,
    pub reactions_count: usize,
    pub custom_content: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ContentPayload {
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ReactionRequest {
    pub content: String,
    pub comment_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ConvType {
    Comment,
    Deploy,
    Commit,
    ForcePush,
    Edit,
    Review,
    Approve,
    MergeQueue,
    Merged,
    Closed,
    Reopen,
    Label,
    Assignee,
    Mention,
}

impl From<ConvTypeEnum> for ConvType {
    fn from(value: ConvTypeEnum) -> Self {
        match value {
            ConvTypeEnum::Comment => ConvType::Comment,
            ConvTypeEnum::Deploy => ConvType::Deploy,
            ConvTypeEnum::Commit => ConvType::Commit,
            ConvTypeEnum::ForcePush => ConvType::ForcePush,
            ConvTypeEnum::Edit => ConvType::Edit,
            ConvTypeEnum::Review => ConvType::Review,
            ConvTypeEnum::Approve => ConvType::Approve,
            ConvTypeEnum::MergeQueue => ConvType::MergeQueue,
            ConvTypeEnum::Merged => ConvType::Merged,
            ConvTypeEnum::Closed => ConvType::Closed,
            ConvTypeEnum::Reopen => ConvType::Reopen,
            ConvTypeEnum::Label => ConvType::Label,
            ConvTypeEnum::Assignee => ConvType::Assignee,
            ConvTypeEnum::Mention => ConvType::Mention,
        }
    }
}
