use sea_orm::entity::prelude::*;

use crate::label::Entity;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ItemLabels,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ItemLabels => Entity::has_many(crate::item_labels::Entity).into(),
        }
    }
}
