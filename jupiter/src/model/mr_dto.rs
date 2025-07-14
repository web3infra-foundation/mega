use callisto::{item_assignees, label, mega_mr};

use crate::model::conv_dto::ConvWithReactions;

pub struct MRDetails {
    pub username: String,
    pub mr: mega_mr::Model,
    pub conversations: Vec<ConvWithReactions>,
    pub labels: Vec<label::Model>,
    pub assignees: Vec<item_assignees::Model>,
}
