use crate::entity_ext::{generate_id, generate_public_id};
use crate::reactions;

impl reactions::Model {
    pub fn new(
        content: Option<String>,
        subject_id: i64,
        subject_type: &str,
        username: &str,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            created_at: now,
            updated_at: now,
            public_id: generate_public_id(),
            content,
            subject_id,
            subject_type: subject_type.to_owned(),
            organization_membership_id: None,
            username: username.to_owned(),
            discarded_at: None,
        }
    }
}
