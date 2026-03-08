use chrono::Utc;

use crate::{
    bots,
    entity_ext::generate_id,
    sea_orm_active_enums::{BotStatusEnum, PermissionScopeEnum},
};

impl bots::Model {
    pub fn new(
        name: String,
        organization_id: Option<i64>,
        creator_user_id: i64,
        permission_scope: PermissionScopeEnum,
        status: BotStatusEnum,
    ) -> Self {
        let now = Utc::now().into();

        Self {
            id: generate_id(),
            name,
            organization_id,
            creator_user_id,
            permission_scope,
            status,
            created_at: now,
            updated_at: now,
        }
    }
}
