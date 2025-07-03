use sea_orm::entity::prelude::*;

use crate::{item_assignees::Column, item_assignees::Entity};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MegaIssue,
    MegaMr,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::MegaIssue => Entity::belongs_to(crate::mega_issue::Entity)
                .from(Column::ItemId)
                .to(crate::mega_issue::Column::Id)
                .into(),
            Self::MegaMr => Entity::belongs_to(crate::mega_mr::Entity)
                .from(Column::ItemId)
                .to(crate::mega_mr::Column::Id)
                .into(),
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
