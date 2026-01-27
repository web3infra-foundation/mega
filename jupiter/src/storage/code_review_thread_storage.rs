use std::ops::Deref;

use callisto::{
    mega_code_review_thread::{self},
    sea_orm_active_enums::{DiffSideEnum, ThreadStatusEnum},
};
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
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

    pub async fn find_thread_by_anchor(
        &self,
        link: &str,
        file_path: &str,
        line_number: i32,
        diff_side: callisto::sea_orm_active_enums::DiffSideEnum,
    ) -> Result<Option<mega_code_review_thread::Model>, MegaError> {
        let thread = mega_code_review_thread::Entity::find()
            .filter(mega_code_review_thread::Column::Link.eq(link))
            .filter(mega_code_review_thread::Column::FilePath.eq(file_path))
            .filter(mega_code_review_thread::Column::LineNumber.eq(line_number))
            .filter(mega_code_review_thread::Column::DiffSide.eq(diff_side))
            .one(self.get_connection())
            .await?;
        Ok(thread)
    }

    pub async fn find_or_create_thread(
        &self,
        link: &str,
        file_path: &str,
        line_number: i32,
        diff_side: DiffSideEnum,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        if let Some(thread) = self
            .find_thread_by_anchor(link, file_path, line_number, diff_side.clone())
            .await?
        {
            Ok(thread)
        } else {
            let new_thread = mega_code_review_thread::Model::new(
                link,
                file_path,
                line_number,
                diff_side,
                ThreadStatusEnum::Open,
            );
            let active_model = new_thread.into_active_model();
            let thread = active_model.insert(self.get_connection()).await?;
            Ok(thread)
        }
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
