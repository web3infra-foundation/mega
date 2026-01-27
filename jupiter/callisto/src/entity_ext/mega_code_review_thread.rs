use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    mega_code_review_thread::{self, Column, Entity},
    sea_orm_active_enums::{DiffSideEnum, ThreadStatusEnum},
};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MegaMr,
    Comment,
}

impl RelationTrait for Relation {
    fn def(&self) -> sea_orm::RelationDef {
        match self {
            Self::MegaMr => Entity::belongs_to(crate::mega_cl::Entity)
                .from(Column::Link)
                .to(crate::mega_cl::Column::Link)
                .into(),
            Self::Comment => Entity::has_many(crate::mega_code_review_comment::Entity).into(),
        }
    }
}

impl Related<crate::mega_cl::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MegaMr.def()
    }
}

impl Related<crate::mega_code_review_comment::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Comment.def()
    }
}

impl mega_code_review_thread::Model {
    pub fn new(
        link: &str,
        file_path: &str,
        line_number: i32,
        diff_side: DiffSideEnum,
        thread_status: ThreadStatusEnum,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: generate_id(),
            link: link.to_owned(),
            file_path: file_path.to_owned(),
            line_number,
            diff_side,
            thread_status,
            created_at: now,
            updated_at: now,
        }
    }
}
