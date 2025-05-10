use chrono::{DateTime, Utc};
use sea_orm::prelude::StringLen;
use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};

use callisto::repo_sync_result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageModel {
    pub db_model: repo_sync_result::Model, // 包装数据库 Model
    pub message_kind: MessageKind,
    pub source_of_data: SourceOfData,
    pub timestamp: DateTime<Utc>, // 消息发送时的时间戳
    pub extra_field: String,      // 可以添加额外字段
}

// 手动实现数据库 Model 的特性
impl MessageModel {
    pub fn new(
        db_model: repo_sync_result::Model,
        message_kind: MessageKind,
        source_of_data: SourceOfData,
        timestamp: DateTime<Utc>,
        extra_field: String,
    ) -> Self {
        Self {
            db_model,
            message_kind,
            source_of_data,
            timestamp,
            extra_field,
        }
    }
}

use std::ops::Deref;
// 通过 Deref 转发数据库 Model 的特性
// 可以直接访问 crate_name：message_model.crate_name，而不用每次写成 message_model.inner.crate_name。
impl Deref for MessageModel {
    type Target = repo_sync_result::Model;

    fn deref(&self) -> &Self::Target {
        &self.db_model
    }
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MessageKind {
    #[sea_orm(string_value = "mega")]
    Mega,
    #[sea_orm(string_value = "user")]
    User,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum SourceOfData {
    #[sea_orm(string_value = "cratesio")]
    Cratesio,
    #[sea_orm(string_value = "github")]
    Github,
}
