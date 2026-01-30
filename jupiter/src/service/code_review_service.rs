use std::{collections::HashMap, vec};

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
        code_review_comment_storage::CodeReviewCommentStorage,
        code_review_thread_storage::CodeReviewThreadStorage,
    },
};

#[derive(Clone)]
pub struct CodeReviewService {
    pub code_review_thread: CodeReviewThreadStorage,
    pub code_review_comment: CodeReviewCommentStorage,
}

impl CodeReviewService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            code_review_thread: CodeReviewThreadStorage {
                base: base_storage.clone(),
            },
            code_review_comment: CodeReviewCommentStorage {
                base: base_storage.clone(),
            },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            code_review_thread: CodeReviewThreadStorage { base: mock.clone() },
            code_review_comment: CodeReviewCommentStorage { base: mock.clone() },
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

        // Batch fetch related entities
        let anchors = self
            .code_review_thread
            .get_anchors_by_thread_ids(&thread_ids)
            .await?;
        let positions = self
            .code_review_thread
            .get_positions_by_thread_ids(&thread_ids)
            .await?;
        let comments = self
            .code_review_comment
            .get_comments_by_thread_ids(&thread_ids)
            .await?;

        // Map entities by thread_id or anchor_id
        let comments_by_thread: HashMap<i64, Vec<_>> =
            comments.into_iter().fold(HashMap::new(), |mut map, c| {
                map.entry(c.thread_id).or_default().push(c);
                map
            });

        let anchors_by_thread: HashMap<i64, Vec<_>> =
            anchors.into_iter().fold(HashMap::new(), |mut map, a| {
                map.entry(a.thread_id).or_default().push(a);
                map
            });

        let positions_by_anchor: HashMap<i64, _> =
            positions.into_iter().map(|p| (p.anchor_id, p)).collect();

        // Build ThreadReviewView
        let mut files_map: HashMap<String, Vec<ThreadReviewView>> = HashMap::new();

        for thread in &threads {
            if let Some(thread_anchors) = anchors_by_thread.get(&thread.id) {
                for anchor in thread_anchors {
                    let position = positions_by_anchor.get(&anchor.id).ok_or_else(|| {
                        MegaError::Other(format!("Position not found for anchor {}", anchor.id))
                    })?;

                    let thread_comments = comments_by_thread
                        .get(&thread.id)
                        .cloned()
                        .unwrap_or_default();

                    let thread_view = ThreadReviewView::from_models(
                        thread.clone(),
                        anchor.clone(),
                        position.clone(),
                        thread_comments,
                    );

                    files_map
                        .entry(anchor.file_path.clone())
                        .or_default()
                        .push(thread_view);
                }
            }
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
        diff_side: DiffSideEnum,
        anchor_commit_sha: &str,
        original_line_number: i32,
        normalized_content: &str,
        context_before: &str,
        context_after: &str,
        user_name: String,
        content: String,
    ) -> Result<ThreadReviewView, MegaError> {
        let (thread, anchor, position) = self
            .code_review_thread
            .create_thread_by_anchor(
                link,
                file_path,
                &diff_side,
                anchor_commit_sha,
                original_line_number,
                normalized_content,
                context_before,
                context_after,
            )
            .await?;

        let comment = self
            .code_review_comment
            .create_code_review_comment(thread.id, user_name, None, Some(content))
            .await?;

        let thread = self.code_review_thread.touch_thread(thread.id).await?;

        Ok(ThreadReviewView::from_models(
            thread,
            anchor,
            position,
            vec![comment],
        ))
    }

    pub async fn reply_to_comment(
        &self,
        thread_id: i64,
        parent_comment_id: i64,
        user_name: String,
        content: String,
    ) -> Result<CommentReviewView, MegaError> {
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
            .create_code_review_comment(
                thread_id,
                user_name,
                Some(parent_comment_id),
                Some(content),
            )
            .await?;

        Ok(comment.into())
    }

    pub async fn update_comment(
        &self,
        comment_id: i64,
        new_content: String,
    ) -> Result<CommentReviewView, MegaError> {
        self.code_review_comment
            .find_comment_by_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        let updated_comment = self
            .code_review_comment
            .update_code_review_comment(comment_id, Some(new_content))
            .await?;

        Ok(updated_comment.into())
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
            .find_comment_by_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        self.code_review_comment
            .delete_comment_by_comment_id(comment_id)
            .await?;
        Ok(comment)
    }
}
