use chrono::Utc;

use crate::{bot_keys, entity_ext::generate_id};

impl bot_keys::Model {
    pub fn new(bot_id: i64, private_key: String, public_key: String) -> Self {
        let now = Utc::now().into();

        Self {
            id: generate_id(),
            bot_id,
            private_key,
            public_key,
            created_at: now,
        }
    }
}
