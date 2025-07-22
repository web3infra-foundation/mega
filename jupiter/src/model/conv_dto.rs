use callisto::{mega_conversation, reactions};

pub struct ConvWithReactions {
    pub conversation: mega_conversation::Model,
    pub reactions: Vec<reactions::Model>,
}
