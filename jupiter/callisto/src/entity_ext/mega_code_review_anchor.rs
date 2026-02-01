use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::{generate_hash_content, generate_id, normalize},
    mega_code_review_anchor::{self, Column, Entity},
    sea_orm_active_enums::DiffSideEnum,
};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Thread,
    Position,
}

impl RelationTrait for Relation {
    fn def(&self) -> sea_orm::RelationDef {
        match self {
            Self::Thread => Entity::belongs_to(crate::mega_code_review_thread::Entity)
                .from(Column::ThreadId)
                .to(crate::mega_code_review_thread::Column::Id)
                .into(),

            Self::Position => Entity::has_one(crate::mega_code_review_position::Entity)
                .from(Column::Id)
                .to(crate::mega_code_review_position::Column::AnchorId)
                .into(),
        }
    }
}

impl mega_code_review_anchor::Model {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_id: i64,
        file_path: &str,
        diff_side: &DiffSideEnum,
        anchor_commit_sha: &str,
        original_line_number: i32,
        normalized_content: &str,
        context_before: &str,
        context_after: &str,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: generate_id(),
            thread_id,
            file_path: file_path.to_owned(),
            diff_side: diff_side.to_owned(),
            anchor_commit_sha: anchor_commit_sha.to_owned(),
            original_line_number,
            normalized_content: normalized_content.to_owned(),
            normalized_hash: generate_hash_content(&normalize(normalized_content)),
            context_before: context_before.to_owned(),
            context_before_hash: generate_hash_content(&normalize(context_before)),
            context_after: context_after.to_owned(),
            context_after_hash: generate_hash_content(&normalize(context_after)),
            created_at: now,
        }
    }
}
