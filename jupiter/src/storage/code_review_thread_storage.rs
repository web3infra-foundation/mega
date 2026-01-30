use std::ops::Deref;

use callisto::{
    entity_ext::generate_hash_content,
    mega_code_review_anchor, mega_code_review_position,
    mega_code_review_thread::{self},
    sea_orm_active_enums::{DiffSideEnum, PositionStatusEnum, ThreadStatusEnum},
};
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, TransactionTrait,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct CodeReviewThreadStorage {
    pub base: BaseStorage,
}

impl Deref for CodeReviewThreadStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl CodeReviewThreadStorage {
    pub async fn get_code_review_threads_by_link(
        &self,
        link: &str,
    ) -> Result<Vec<mega_code_review_thread::Model>, MegaError> {
        let threads = mega_code_review_thread::Entity::find()
            .filter(mega_code_review_thread::Column::Link.eq(link))
            .all(self.get_connection())
            .await?;

        Ok(threads)
    }

    pub async fn find_thread_by_id(
        &self,
        thread_id: i64,
    ) -> Result<Option<mega_code_review_thread::Model>, MegaError> {
        let thread = mega_code_review_thread::Entity::find_by_id(thread_id)
            .one(self.get_connection())
            .await?;

        Ok(thread)
    }

    pub async fn get_anchors_by_thread_ids(
        &self,
        thread_ids: &[i64],
    ) -> Result<Vec<mega_code_review_anchor::Model>, MegaError> {
        let anchors = mega_code_review_anchor::Entity::find()
            .filter(mega_code_review_anchor::Column::ThreadId.is_in(thread_ids.to_vec()))
            .order_by_asc(mega_code_review_anchor::Column::CreatedAt)
            .all(self.get_connection())
            .await?;

        Ok(anchors)
    }

    pub async fn get_positions_by_thread_ids(
        &self,
        thread_ids: &[i64],
    ) -> Result<Vec<mega_code_review_position::Model>, MegaError> {
        let anchors = self.get_anchors_by_thread_ids(thread_ids).await?;
        if anchors.is_empty() {
            return Ok(vec![]);
        }

        let anchor_ids: Vec<i64> = anchors.iter().map(|a| a.id).collect();

        let positions = mega_code_review_position::Entity::find()
            .filter(mega_code_review_position::Column::AnchorId.is_in(anchor_ids))
            .order_by_asc(mega_code_review_position::Column::UpdatedAt)
            .all(self.get_connection())
            .await?;

        Ok(positions)
    }

    pub async fn find_thread_by_anchor(
        &self,
        file_path: &str,
        diff_side: &DiffSideEnum,
        anchor_commit_sha: &str,
        normalized_content: &str,
        context_before: &str,
        context_after: &str,
    ) -> Result<Option<mega_code_review_thread::Model>, MegaError> {
        let anchor = mega_code_review_anchor::Entity::find()
            .filter(mega_code_review_anchor::Column::FilePath.eq(file_path))
            .filter(mega_code_review_anchor::Column::DiffSide.eq(diff_side.to_owned()))
            .filter(mega_code_review_anchor::Column::AnchorCommitSha.eq(anchor_commit_sha))
            .filter(
                mega_code_review_anchor::Column::NormalizedHash
                    .eq(generate_hash_content(normalized_content)),
            )
            .filter(
                mega_code_review_anchor::Column::ContextBeforeHash
                    .eq(generate_hash_content(context_before)),
            )
            .filter(
                mega_code_review_anchor::Column::ContextAfterHash
                    .eq(generate_hash_content(context_after)),
            )
            .one(self.get_connection())
            .await?;

        match anchor {
            Some(anchor) => self.find_thread_by_id(anchor.thread_id).await,
            None => Ok(None),
        }
    }

    pub async fn create_thread_by_anchor(
        &self,
        link: &str,
        file_path: &str,
        diff_side: &DiffSideEnum,
        anchor_commit_sha: &str,
        original_line_number: i32,
        normalized_content: &str,
        context_before: &str,
        context_after: &str,
    ) -> Result<
        (
            mega_code_review_thread::Model,
            mega_code_review_anchor::Model,
            mega_code_review_position::Model,
        ),
        MegaError,
    > {
        // Check if a thread already exists
        if let Some(existing_thread) = self
            .find_thread_by_anchor(
                file_path,
                diff_side,
                anchor_commit_sha,
                normalized_content,
                context_before,
                context_after,
            )
            .await?
        {
            if existing_thread.thread_status == ThreadStatusEnum::Open {
                return Err(MegaError::Other(format!(
                    "Thread with id {} already exists",
                    existing_thread.id
                )));
            }
        }

        // Begin transaction
        let txn = self.get_connection().begin().await?;

        // Insert thread
        let new_thread = mega_code_review_thread::Model::new(link, ThreadStatusEnum::Open);
        let thread = new_thread.into_active_model().insert(&txn).await?;

        // Insert anchor
        let new_anchor = mega_code_review_anchor::Model::new(
            thread.id,
            file_path,
            diff_side,
            anchor_commit_sha,
            original_line_number,
            normalized_content,
            context_before,
            context_after,
        );
        let anchor = new_anchor.into_active_model().insert(&txn).await?;

        // Insert position
        let new_position = mega_code_review_position::Model::new(
            anchor.id,
            anchor_commit_sha,
            file_path,
            diff_side,
            original_line_number,
            100,
            PositionStatusEnum::Ok,
        );
        let position = new_position.into_active_model().insert(&txn).await?;

        // Commit transaction
        txn.commit().await?;

        Ok((thread, anchor, position))
    }

    pub async fn touch_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        let thread = mega_code_review_thread::Entity::find_by_id(thread_id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {thread_id} not found")))?;
        let mut active: mega_code_review_thread::ActiveModel = thread.into();
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = active.update(self.get_connection()).await?;
        Ok(updated)
    }

    pub async fn update_code_review_thread_status(
        &self,
        thread_id: i64,
        thread_status: ThreadStatusEnum,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        let thread = mega_code_review_thread::Entity::find_by_id(thread_id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread with id `{thread_id}` not found")))?;

        let mut active_thread: mega_code_review_thread::ActiveModel = thread.into();

        active_thread.thread_status = Set(thread_status);
        active_thread.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated_thread = active_thread.update(self.get_connection()).await?;

        Ok(updated_thread)
    }

    pub async fn delete_code_review_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        let model = mega_code_review_thread::Entity::find_by_id(thread_id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread with id `{thread_id}` not found")))?;

        let delete_result = mega_code_review_thread::Entity::delete_by_id(thread_id)
            .exec(self.get_connection())
            .await
            .map_err(|e| MegaError::Other(format!("Failed to delete code review thread: {e}")))?;

        if delete_result.rows_affected == 0 {
            return Err(MegaError::Other(format!(
                "Thread with id `{thread_id}` was not deleted"
            )));
        }

        Ok(model)
    }
}
