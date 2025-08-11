use sea_orm::entity::prelude::*;

use crate::{
    entity_ext::generate_id,
    mega_mr::{self, Entity},
    sea_orm_active_enums::MergeStatusEnum,
};

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
        Some(crate::entity_ext::item_labels::Relation::MegaMr.def().rev())
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

impl mega_mr::Model {
    pub fn new(
        path: String,
        title: String,
        link: String,
        from_hash: String,
        to_hash: String,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            link,
            title: title.to_owned(),
            status: MergeStatusEnum::Open,
            created_at: now,
            updated_at: now,
            merge_date: None,
            path,
            from_hash,
            to_hash,
        }
    }
}
