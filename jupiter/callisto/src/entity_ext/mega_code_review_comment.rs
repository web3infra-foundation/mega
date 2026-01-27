use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    mega_code_review_comment::{self, Column, Entity},
};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Thread,
}

impl RelationTrait for Relation {
    fn def(&self) -> sea_orm::RelationDef {
        match self {
            Self::Thread => Entity::belongs_to(crate::mega_code_review_thread::Entity)
                .from(Column::ThreadId)
                .to(crate::mega_code_review_thread::Column::Id)
                .into(),
        }
    }
}

impl Related<crate::mega_code_review_thread::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Thread.def()
    }
}

impl mega_code_review_comment::Model {
    pub fn new(
        thread_id: i64,
        parent_id: Option<i64>,
        user_name: String,
        content: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: generate_id(),
            thread_id,
            parent_id,
            user_name,
            content,
            created_at: now,
            updated_at: now,
        }
    }
}
