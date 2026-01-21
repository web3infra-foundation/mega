use std::collections::HashMap;

use callisto::{
    mega_code_review_comment, mega_code_review_thread,
    sea_orm_active_enums::{DiffSideEnum, ThreadStatusEnum},
};
use common::errors::MegaError;

use crate::{
    model::code_review_dto::{
        CodeReviewViews, CommentReviewView, FileReviewView, ThreadReviewView,
    },
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        code_review_comment_storage::CodeReivewCommentStorage,
        code_review_thread_storage::CodeReviewThreadStorage,
    },
};

#[derive(Clone)]
pub struct CodeReviewService {
    pub code_review_thread: CodeReviewThreadStorage,
    pub code_review_comment: CodeReivewCommentStorage,
}

impl CodeReviewService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            code_review_thread: CodeReviewThreadStorage {
                base: base_storage.clone(),
            },
            code_review_comment: CodeReivewCommentStorage {
                base: base_storage.clone(),
            },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            code_review_thread: CodeReviewThreadStorage { base: mock.clone() },
            code_review_comment: CodeReivewCommentStorage { base: mock.clone() },
        }
    }

    pub async fn get_all_comments_by_link(&self, link: &str) -> Result<CodeReviewViews, MegaError> {
        let threads = self
            .code_review_thread
            .get_code_review_threads_by_link(link)
            .await?;

        if threads.is_empty() {
            return Ok(CodeReviewViews {
                link: link.to_string(),
                files: vec![],
            });
        }

        let thread_ids: Vec<i64> = threads.iter().map(|t| t.id).collect();

        let comments = self
            .code_review_comment
            .get_comments_by_thread_ids(&thread_ids)
            .await?;

        let mut comments_by_thread: HashMap<i64, Vec<CommentReviewView>> = HashMap::new();

        for c in comments {
            comments_by_thread
                .entry(c.thread_id)
                .or_default()
                .push(CommentReviewView {
                    comment_id: c.id,
                    user_id: c.user_id,
                    content: c.content,
                    parent_id: c.parent_id,
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                });
        }

        let mut files_map: HashMap<String, Vec<ThreadReviewView>> = HashMap::new();

        for t in threads {
            let thread_view = ThreadReviewView {
                thread_id: t.id,
                line_number: t.line_number,
                diff_side: t.diff_side,
                status: t.thread_status,
                created_at: t.created_at,
                updated_at: t.updated_at,
                comments: comments_by_thread.remove(&t.id).unwrap_or_default(),
            };

            files_map
                .entry(t.file_path.clone())
                .or_default()
                .push(thread_view);
        }

        let files = files_map
            .into_iter()
            .map(|(file_path, threads)| FileReviewView { file_path, threads })
            .collect();

        Ok(CodeReviewViews {
            link: link.to_string(),
            files,
        })
    }

    pub async fn create_inline_comment(
        &self,
        link: &str,
        file_path: &str,
        line_number: i32,
        diff_side: DiffSideEnum,
        user_id: i64,
        content: String,
    ) -> Result<ThreadReviewView, MegaError> {
        let thread = match self
            .code_review_thread
            .find_or_create_thread(link, file_path, line_number, diff_side.clone())
            .await?
        {
            t => t,
        };

        let comment = self
            .code_review_comment
            .create_code_review_comment(thread.id, user_id, None, Some(content))
            .await?;

        let thread = self.code_review_thread.touch_thread(thread.id).await?;

        Ok(ThreadReviewView {
            thread_id: thread.id,
            line_number: thread.line_number,
            diff_side: thread.diff_side,
            status: thread.thread_status,
            created_at: thread.created_at,
            updated_at: thread.updated_at,
            comments: vec![comment.into()],
        })
    }

    pub async fn reply_to_comment(
        &self,
        thread_id: i64,
        parent_comment_id: i64,
        user_id: i64,
        content: String,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let parent_comment = self
            .code_review_comment
            .find_comment_by_id(parent_comment_id)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!("Parent comment {} not found", parent_comment_id))
            })?;

        if parent_comment.thread_id != thread_id {
            return Err(MegaError::Other(
                "Parent comment does not belong to the thread".to_string(),
            ));
        }

        let comment = self
            .code_review_comment
            .create_code_review_comment(thread_id, user_id, Some(parent_comment_id), Some(content))
            .await?;

        Ok(comment)
    }

    pub async fn update_comment(
        &self,
        comment_id: i64,
        new_content: String,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        self.code_review_comment
            .find_comment_by_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        let updated_comment = self
            .code_review_comment
            .update_code_review_comment(comment_id, Some(new_content))
            .await?;

        Ok(updated_comment)
    }

    pub async fn resolve_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let updated_thread = self
            .code_review_thread
            .update_code_review_thread_status(thread_id, ThreadStatusEnum::Resolved)
            .await?;

        Ok(updated_thread)
    }

    pub async fn reopen_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let updated_thread = self
            .code_review_thread
            .update_code_review_thread_status(thread_id, ThreadStatusEnum::Open)
            .await?;

        Ok(updated_thread)
    }

    pub async fn delete_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        let thread = self
            .code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        self.code_review_comment
            .delete_comments_by_thread_id(thread_id)
            .await?;
        self.code_review_thread
            .delete_code_review_thread(thread_id)
            .await?;

        Ok(thread)
    }

    pub async fn delete_comment(
        &self,
        comment_id: i64,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        let comment = self
            .code_review_comment
            .get_comment_by_comment_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        self.code_review_comment
            .delete_comment_by_comment_id(comment_id)
            .await?;
        Ok(comment)
    }
}
