use std::ops::Deref;

use callisto::{
    audit_logs,
    sea_orm_active_enums::{ActorTypeEnum, AuditActionEnum, TargetTypeEnum},
};
use chrono::Utc;
use common::errors::MegaError;
use idgenerator::IdInstance;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, prelude::DateTimeWithTimeZone};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct AuditStorage {
    pub base: BaseStorage,
}

impl Deref for AuditStorage {
    type Target = BaseStorage;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl AuditStorage {
    /// Write an audit log entry for a given actor and target.
    ///
    /// `metadata` is stored as JSON for flexible, structured details per action.
    pub async fn log_audit(
        &self,
        actor_id: i64,
        actor_type: ActorTypeEnum,
        action: AuditActionEnum,
        target_type: TargetTypeEnum,
        target_id: i64,
        metadata: Option<serde_json::Value>,
    ) -> Result<audit_logs::Model, MegaError> {
        let created_at: DateTimeWithTimeZone = Utc::now().into();
        let model = audit_logs::ActiveModel {
            id: Set(IdInstance::next_id()),
            actor_id: Set(actor_id),
            actor_type: Set(actor_type),
            action: Set(action),
            target_type: Set(target_type),
            target_id: Set(target_id),
            metadata: Set(metadata),
            created_at: Set(created_at),
        };

        let inserted = model.insert(self.get_connection()).await?;
        Ok(inserted)
    }
}
