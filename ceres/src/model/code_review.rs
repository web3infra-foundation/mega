use callisto::{
    mega_code_review_comment, mega_code_review_thread,
    sea_orm_active_enums::{DiffSideEnum, ThreadStatusEnum},
};
use jupiter::model::code_review_dto::{
    CodeReviewViews, CommentReviewView, FileReviewView, ThreadReviewView,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct InitializeCommentRequest {
    pub file_path: String,
    pub line_number: i32,
    pub diff_side: DiffSide,
    pub content: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CommentReplyRequest {
    pub parent_comment_id: i64,
    pub content: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateCommentRequest {
    pub comtent_id: i64,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DiffSide {
    Old,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ThreadStatus {
    Open,
    Resolved,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CodeReviewResponse {
    pub link: String,
    pub files: Vec<FileReviewResponse>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct FileReviewResponse {
    pub file_path: String,
    pub threads: Vec<ThreadReviewResponse>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ThreadReviewResponse {
    pub thread_id: i64,
    pub line_number: i32,
    pub diff_side: DiffSide,
    pub status: ThreadStatus,
    pub created_at: String,
    pub updated_at: String,
    pub comments: Vec<CommentReviewResponse>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ThreadStatusResponse {
    pub thread_id: i64,
    pub link: String,
    pub file_path: String,
    pub line_number: i32,
    pub diff_side: DiffSide,
    pub status: ThreadStatus,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CommentReviewResponse {
    pub comment_id: i64,
    pub user_name: String,
    pub content: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<mega_code_review_comment::Model> for CommentReviewResponse {
    fn from(value: mega_code_review_comment::Model) -> Self {
        Self {
            comment_id: value.id,
            user_name: value.user_name,
            content: value.content,
            parent_id: value.parent_id,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
        }
    }
}

impl From<CommentReviewView> for CommentReviewResponse {
    fn from(value: CommentReviewView) -> Self {
        Self {
            comment_id: value.comment_id,
            user_name: value.user_name,
            content: value.content,
            parent_id: value.parent_id,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
        }
    }
}

impl From<ThreadReviewView> for ThreadReviewResponse {
    fn from(value: ThreadReviewView) -> Self {
        Self {
            thread_id: value.thread_id,
            line_number: value.line_number,
            diff_side: value.diff_side.into(),
            status: value.status.into(),
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
            comments: value.comments.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<mega_code_review_thread::Model> for ThreadStatusResponse {
    fn from(value: mega_code_review_thread::Model) -> Self {
        Self {
            thread_id: value.id,
            link: value.link,
            file_path: value.file_path,
            line_number: value.line_number,
            diff_side: value.diff_side.into(),
            status: value.thread_status.into(),
        }
    }
}

impl From<FileReviewView> for FileReviewResponse {
    fn from(value: FileReviewView) -> Self {
        Self {
            file_path: value.file_path,
            threads: value.threads.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<CodeReviewViews> for CodeReviewResponse {
    fn from(value: CodeReviewViews) -> Self {
        Self {
            link: value.link,
            files: value.files.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DiffSideEnum> for DiffSide {
    fn from(value: DiffSideEnum) -> Self {
        match value {
            DiffSideEnum::Old => DiffSide::Old,
            DiffSideEnum::New => DiffSide::New,
        }
    }
}

impl From<DiffSide> for DiffSideEnum {
    fn from(value: DiffSide) -> Self {
        match value {
            DiffSide::Old => DiffSideEnum::Old,
            DiffSide::New => DiffSideEnum::New,
        }
    }
}

impl From<ThreadStatusEnum> for ThreadStatus {
    fn from(value: ThreadStatusEnum) -> Self {
        match value {
            ThreadStatusEnum::Open => ThreadStatus::Open,
            ThreadStatusEnum::Resolved => ThreadStatus::Resolved,
        }
    }
}

impl From<ThreadStatus> for ThreadStatusEnum {
    fn from(value: ThreadStatus) -> Self {
        match value {
            ThreadStatus::Open => ThreadStatusEnum::Open,
            ThreadStatus::Resolved => ThreadStatusEnum::Resolved,
        }
    }
}
