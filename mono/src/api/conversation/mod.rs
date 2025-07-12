use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use callisto::{reactions, sea_orm_active_enums::ConvTypeEnum};
use jupiter::model::issue_dto::ConvWithReactions;

pub mod conv_router;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConversationItem {
    pub id: i64,
    pub username: String,
    pub conv_type: ConvTypeEnum,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub grouped_reactions: Vec<ReactionItem>,
}

impl From<ConvWithReactions> for ConversationItem {
    fn from(value: ConvWithReactions) -> Self {
        Self {
            id: value.conversation.id,
            username: value.conversation.username,
            conv_type: value.conversation.conv_type,
            comment: value.conversation.comment,
            created_at: value.conversation.created_at.and_utc().timestamp(),
            updated_at: value.conversation.updated_at.and_utc().timestamp(),
            grouped_reactions: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ReactionItem {
    pub viewer_reaction_id: String,
    pub emoji: String,
    pub tooltip: String,
    pub reactions_count: usize,
    pub custom_content: String,
}
impl From<reactions::Model> for ConversationItem {
    fn from(_: reactions::Model) -> Self {
        todo!()
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SaveCommentRequest {
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ReactionRequest {
    pub content: String,
    pub comment_type: String,
}
