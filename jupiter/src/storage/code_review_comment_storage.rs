use std::ops::Deref;

use callisto::mega_code_review_comment;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct CodeReivewCommentStorage {
    pub base: BaseStorage,
}

impl Deref for CodeReivewCommentStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl CodeReivewCommentStorage {
    pub async fn get_comment_by_comment_id(
        &self,
        comment_id: i64,
    ) -> Result<Option<mega_code_review_comment::Model>, MegaError> {
        let comment = mega_code_review_comment::Entity::find_by_id(comment_id)
            .one(self.get_connection())
            .await?;

        Ok(comment)
    }

    pub async fn get_comments_by_thread_ids(
        &self,
        thread_ids: &[i64],
    ) -> Result<Vec<mega_code_review_comment::Model>, MegaError> {
        let comments = mega_code_review_comment::Entity::find()
            .filter(mega_code_review_comment::Column::ThreadId.is_in(thread_ids.to_vec()))
            .order_by_asc(mega_code_review_comment::Column::CreatedAt)
            .all(self.get_connection())
            .await?;

        Ok(comments)
    }

    pub async fn find_comment_by_id(
        &self,
        comment_id: i64,
    ) -> Result<Option<mega_code_review_comment::Model>, MegaError> {
        let comment = mega_code_review_comment::Entity::find_by_id(comment_id)
            .one(self.get_connection())
            .await?;

        Ok(comment)
    }

    pub async fn create_code_review_comment(
        &self,
        thread_id: i64,
        user_name: String,
        parent_id: Option<i64>,
        content: Option<String>,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        let comment =
            mega_code_review_comment::Model::new(thread_id, parent_id, user_name, content);
        let active_comment = comment.into_active_model();
        let res = active_comment.insert(self.get_connection()).await.unwrap();
        Ok(res)
    }

    pub async fn update_code_review_comment(
        &self,
        comment_id: i64,
        comment: Option<String>,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        let model = mega_code_review_comment::Entity::find_by_id(comment_id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment with id `{comment_id}` not found")))?;

        let mut active_comment: mega_code_review_comment::ActiveModel = model.into();

        active_comment.content = Set(comment);

        let updated_comment = active_comment.update(self.get_connection()).await?;

        Ok(updated_comment)
    }

    pub async fn delete_comments_by_thread_id(&self, thread_id: i64) -> Result<(), MegaError> {
        mega_code_review_comment::Entity::delete_many()
            .filter(mega_code_review_comment::Column::ThreadId.eq(thread_id))
            .exec(self.get_connection())
            .await
            .map_err(|e| MegaError::Other(format!("Failed to delete code review comments: {e}")))?;

        Ok(())
    }

    pub async fn delete_comment_by_comment_id(&self, comment_id: i64) -> Result<(), MegaError> {
        mega_code_review_comment::Entity::delete_by_id(comment_id)
            .exec(self.get_connection())
            .await
            .map_err(|e| {
                MegaError::Other(format!(
                    "Failed to delete code review comment (id: {comment_id}): {e}"
                ))
            })?;

        Ok(())
    }
}
