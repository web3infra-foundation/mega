use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    mega_code_review_position::{self, Column, Entity},
    sea_orm_active_enums::{DiffSideEnum, PositionStatusEnum},
};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Anchor,
}

impl RelationTrait for Relation {
    fn def(&self) -> sea_orm::RelationDef {
        match self {
            Self::Anchor => Entity::belongs_to(crate::mega_code_review_anchor::Entity)
                .from(Column::AnchorId)
                .to(crate::mega_code_review_anchor::Column::Id)
                .into(),
        }
    }
}

impl mega_code_review_position::Model {
    pub fn new(
        anchor_id: i64,
        commit_sha: &str,
        file_path: &str,
        diff_side: &DiffSideEnum,
        line_number: i32,
        confidence: i32,
        position_status: PositionStatusEnum,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: generate_id(),
            anchor_id,
            commit_sha: commit_sha.to_owned(),
            file_path: file_path.to_owned(),
            diff_side: diff_side.to_owned(),
            line_number,
            confidence,
            position_status,
            created_at: now,
            updated_at: now,
        }
    }
}
