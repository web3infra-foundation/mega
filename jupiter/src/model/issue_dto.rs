use callisto::{item_assignees, label, mega_conversation, mega_issue, reactions};

pub struct IssueDetails {
    pub issue: mega_issue::Model,
    pub conversations: Vec<ConvWithReactions>,
    pub labels: Vec<label::Model>,
    pub assignees: Vec<item_assignees::Model>,
}

pub struct ConvWithReactions {
    pub conversation: mega_conversation::Model,
    pub reactions: Vec<reactions::Model>,
}
