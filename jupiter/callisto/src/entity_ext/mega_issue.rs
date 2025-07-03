use sea_orm::entity::prelude::*;

use crate::mega_issue::Entity;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ItemLabels,
    ItemAssignees,
    Conversation,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ItemLabels => Entity::has_many(crate::item_labels::Entity).into(),
            Self::ItemAssignees => Entity::has_many(crate::item_assignees::Entity).into(),
            Self::Conversation => Entity::has_many(crate::mega_conversation::Entity).into(),
        }
    }
}

impl Related<crate::label::Entity> for Entity {
    fn to() -> RelationDef {
        crate::entity_ext::item_labels::Relation::Label.def()
    }

    fn via() -> Option<RelationDef> {
        Some(
            crate::entity_ext::item_labels::Relation::MegaIssue
                .def()
                .rev(),
        )
    }
}

impl Related<crate::item_assignees::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ItemAssignees.def()
    }
    fn via() -> Option<RelationDef> {
        None
    }
}

impl Related<crate::mega_conversation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
    fn via() -> Option<RelationDef> {
        None
    }
}
