#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct VerifyMrPayload {
    pub assignees: Vec<String>,
}
