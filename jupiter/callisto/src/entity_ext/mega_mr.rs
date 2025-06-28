use sea_orm::entity::prelude::*;

use crate::mega_mr::Entity;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Label,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Label => Entity::has_many(crate::item_labels::Entity).into(),
        }
    }
}

impl Related<crate::label::Entity> for Entity {
    fn to() -> RelationDef {
        crate::entity_ext::item_labels::Relation::Label.def()
    }

    fn via() -> Option<RelationDef> {
        Some(crate::entity_ext::item_labels::Relation::MegaMr.def().rev())
    }
}
