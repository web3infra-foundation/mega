use crate::storage::{conversation_storage::ConversationStorage, issue_storage::IssueStorage};

pub mod issue_service;

#[derive(Clone)]
pub struct IssueService {
    pub issue_storage: IssueStorage,
    pub conversation_storage: ConversationStorage,
}
