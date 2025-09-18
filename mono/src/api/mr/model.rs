#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
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
    pub state: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ChangeReviewStatePayload {
    pub review_id: i64,
    pub new_state: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewerInfo {
    pub username: String,
    pub approved: bool,
}
