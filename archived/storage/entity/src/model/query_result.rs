use sea_orm::FromQueryResult;

#[derive(Debug, FromQueryResult)]
pub struct SelectResult {
    pub node_type: String,
    pub count: i64,
}