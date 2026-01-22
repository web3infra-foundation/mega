use callisto::{
    mega_code_review_comment,
    sea_orm_active_enums::{DiffSideEnum, ThreadStatusEnum},
};

pub struct CodeReviewViews {
    pub link: String,
    pub files: Vec<FileReviewView>,
}

pub struct FileReviewView {
    pub file_path: String,
    pub threads: Vec<ThreadReviewView>,
}

pub struct ThreadReviewView {
    pub thread_id: i64,
    pub line_number: i32,
    pub diff_side: DiffSideEnum,
    pub status: ThreadStatusEnum,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub comments: Vec<CommentReviewView>,
}

pub struct CommentReviewView {
    pub comment_id: i64,
    pub user_name: String,
    pub content: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<mega_code_review_comment::Model> for CommentReviewView {
    fn from(value: mega_code_review_comment::Model) -> Self {
        Self {
            comment_id: value.id,
            user_name: value.user_name,
            content: value.content,
            parent_id: value.parent_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
