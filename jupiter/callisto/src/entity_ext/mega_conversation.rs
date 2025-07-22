use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    mega_conversation::{self, Column, Entity},
    sea_orm_active_enums::ConvTypeEnum,
};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MegaIssue,
    MegaMr,
    Reactions,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::MegaIssue => Entity::belongs_to(crate::mega_issue::Entity)
                .from(Column::Link)
                .to(crate::mega_issue::Column::Link)
                .into(),
            Self::MegaMr => Entity::belongs_to(crate::mega_mr::Entity)
                .from(Column::Link)
                .to(crate::mega_mr::Column::Link)
                .into(),
            Self::Reactions => Entity::has_many(crate::reactions::Entity).into(),
        }
    }
}

impl Related<crate::mega_issue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MegaIssue.def()
    }
}

impl Related<crate::mega_mr::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MegaMr.def()
    }
}

impl Related<crate::reactions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Reactions.def()
    }
    fn via() -> Option<RelationDef> {
        None
    }
}

impl mega_conversation::Model {
    pub fn new(
        link: &str,
        conv_type: ConvTypeEnum,
        comment: Option<String>,
        username: &str,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            link: link.to_owned(),
            conv_type,
            comment,
            created_at: now,
            updated_at: now,
            username: username.to_owned(),
        }
    }
}
