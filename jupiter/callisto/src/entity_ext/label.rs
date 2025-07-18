use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    label::{self, Entity},
};

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

impl label::Model {
    pub fn new(name: &str, color: &str, description: &str) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            created_at: now,
            updated_at: now,
            name: name.to_owned(),
            color: color.to_owned(),
            description: description.to_owned(),
        }
    }
}
