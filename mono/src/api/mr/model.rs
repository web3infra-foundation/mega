#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct VerifyMrPayload {
    pub assignees: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewerPayload {
    pub reviewer_usernames: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewersResponse {
    pub result: Vec<ReviewerInfo>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ChangeReviewerStatePayload {
    pub approved: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ChangeReviewStatePayload {
    pub conversation_id: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewerInfo {
    pub username: String,
    pub approved: bool,
}
