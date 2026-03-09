use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mega_webhook")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub target_url: String,
    pub secret: String,
    #[sea_orm(column_type = "Text")]
    pub event_types: String,
    pub path_filter: Option<String>,
    pub active: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    WebhookEventTypes,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::WebhookEventTypes => {
                Entity::has_many(super::mega_webhook_event_type::Entity).into()
            }
        }
    }
}

impl Related<super::mega_webhook_event_type::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WebhookEventTypes.def()
    }

    fn via() -> Option<RelationDef> {
        None
    }
}

impl ActiveModelBehavior for ActiveModel {}
