use chrono::{DateTime, Utc};
use entity::{contributor_location, github_user};
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

// GitHub用户信息结构
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub bio: Option<String>,
    pub public_repos: Option<i32>,
    pub followers: Option<i32>,
    pub following: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "type")]
    pub user_type: String,
}

impl GitHubUser {
    pub fn is_bot(&self) -> bool {
        self.user_type == "Bot"
    }
}

// 转换函数，用于将GitHub API返回的用户转换为数据库模型
impl From<GitHubUser> for github_user::ActiveModel {
    fn from(user: GitHubUser) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: NotSet,
            github_id: Set(user.id),
            login: Set(user.login),
            name: Set(user.name),
            email: Set(user.email),
            avatar_url: Set(user.avatar_url),
            company: Set(user.company),
            location: Set(user.location),
            bio: Set(user.bio),
            public_repos: Set(user.public_repos),
            followers: Set(user.followers),
            following: Set(user.following),
            created_at: Set(user.created_at.naive_utc()),
            updated_at: Set(user.updated_at.naive_utc()),
            inserted_at: Set(now),
            updated_at_local: Set(now),
            commit_email: NotSet,
        }
    }
}

impl From<github_user::Model> for GitHubUser {
    fn from(value: github_user::Model) -> Self {
        Self {
            id: value.github_id,
            login: value.login,
            avatar_url: value.avatar_url,
            name: value.name,
            email: value.email,
            company: value.company,
            location: value.location,
            bio: value.bio,
            public_repos: value.public_repos,
            followers: value.followers,
            following: value.following,
            user_type: "User".to_owned(),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(value.updated_at, Utc),
        }
    }
}

pub struct AnalyzedUser {
    pub user_id: i32,
    pub github_id: i64,
    pub login: String,
    pub profile_email: Option<String>,
    pub commit_email: Option<String>,
}

impl From<github_user::Model> for AnalyzedUser {
    fn from(value: github_user::Model) -> Self {
        Self {
            user_id: value.id,
            github_id: value.github_id,
            login: value.login,
            profile_email: value.email,
            commit_email: value.commit_email,
        }
    }
}
// 贡献者信息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contributor {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
    pub contributions: i32,
    pub email: Option<String>,
}

// 贡献者分析结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContributorAnalysis {
    pub has_china_timezone: bool,
    pub common_timezone: String,
}

// 转换函数，将分析结果转换为数据库模型
impl From<&ContributorAnalysis> for contributor_location::ActiveModel {
    fn from(analysis: &ContributorAnalysis) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: NotSet,
            is_from_china: Set(analysis.common_timezone == "+08:00"),
            common_timezone: Set(Some(analysis.common_timezone.clone())),
            analyzed_at: Set(now),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub name: String,
    pub url: String,
    pub created_at: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLResponse {
    pub data: Option<SearchData>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchData {
    pub search: SearchResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub edges: Vec<Edge>,
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub node: Repository,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

// 解析提交数据
#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub login: String,
    pub id: i64,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitInfo {
    pub _author: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitDetail {
    pub author: Option<CommitInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CommitData {
    pub author: Option<CommitAuthor>,
    pub commit: CommitDetail,
}
