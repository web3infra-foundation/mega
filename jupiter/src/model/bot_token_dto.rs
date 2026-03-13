use sea_orm::prelude::DateTimeWithTimeZone;

#[derive(Debug, Clone)]
pub struct BotTokenInfo {
    pub id: i64,
    pub token_name: String,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub revoked: bool,
    pub created_at: DateTimeWithTimeZone,
}

