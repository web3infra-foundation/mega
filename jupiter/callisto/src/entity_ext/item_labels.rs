use sea_orm::entity::prelude::*;

use crate::{item_labels::Column, item_labels::Entity};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MegaIssue,
    MegaMr,
    Label,
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
            Self::Label => Entity::belongs_to(crate::label::Entity)
                .from(Column::LabelId)
                .to(crate::label::Column::Id)
                .into(),
        }
    }
}

impl Related<crate::label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Label.def()
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
