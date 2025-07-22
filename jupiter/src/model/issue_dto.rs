use callisto::{item_assignees, label, mega_issue};

use crate::model::conv_dto::ConvWithReactions;

pub struct IssueDetails {
    pub username: String,
    pub issue: mega_issue::Model,
    pub conversations: Vec<ConvWithReactions>,
    pub labels: Vec<label::Model>,
    pub assignees: Vec<item_assignees::Model>,
}
