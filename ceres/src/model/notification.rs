use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationEventTypeInfo {
    pub code: String,
    pub category: String,
    pub description: String,
    pub system_required: bool,
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserNotificationPreferenceItem {
    pub event_type_code: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserNotificationConfig {
    pub enabled: bool,
    pub delivery_mode: String,
    pub email: String,
    pub preferences: Vec<UserNotificationPreferenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserNotificationConfig {
    pub enabled: Option<bool>,
    pub delivery_mode: Option<String>,
    pub preferences: Option<Vec<UserNotificationPreferenceItem>>,
}
