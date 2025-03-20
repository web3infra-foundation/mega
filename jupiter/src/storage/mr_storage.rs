use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use callisto::{mega_conversation, mega_mr};
use common::errors::MegaError;
use common::utils::generate_id;

#[derive(Clone)]
pub struct MrStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl MrStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        MrStorage { connection }
    }

    pub fn mock() -> Self {
        MrStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn get_open_mr_by_path(
        &self,
        path: &str,
    ) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Path.eq(path))
            .filter(mega_mr::Column::Status.eq(MergeStatusEnum::Open))
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn get_mr_by_status(
        &self,
        status: Vec<MergeStatusEnum>,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<mega_mr::Model>, u64), MegaError> {
        let paginator = mega_mr::Entity::find()
            .filter(mega_mr::Column::Status.is_in(status))
            .order_by_desc(mega_mr::Column::CreatedAt)
            .paginate(self.get_connection(), per_page);
        let num_pages = paginator.num_items().await?;
        Ok(paginator
            .fetch_page(page - 1)
            .await
            .map(|m| (m, num_pages))?)
    }

    pub async fn get_mr(&self, link: &str) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Link.eq(link))
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn save_mr(&self, mr: mega_mr::Model) -> Result<(), MegaError> {
        let a_model = mr.into_active_model();
        a_model.insert(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn close_mr(
        &self,
        model: mega_mr::Model,
        user_id: i64,
        username: &str,
    ) -> Result<(), MegaError> {
        self.update_mr(model.clone()).await.unwrap();
        self.add_mr_conversation(
            &model.link,
            user_id,
            ConvTypeEnum::Closed,
            Some(format!("{} closed this", username)),
        )
        .await
        .unwrap();
        Ok(())
    }

    pub async fn reopen_mr(
        &self,
        model: mega_mr::Model,
        user_id: i64,
        username: &str,
    ) -> Result<(), MegaError> {
        self.update_mr(model.clone()).await.unwrap();
        self.add_mr_conversation(
            &model.link,
            user_id,
            ConvTypeEnum::Reopen,
            Some(format!("{} reopen this", username)),
        )
        .await
        .unwrap();
        Ok(())
    }

    pub async fn update_mr(&self, mr: mega_mr::Model) -> Result<(), MegaError> {
        let mut a_model = mr.into_active_model();
        a_model = a_model.reset_all();
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn get_mr_conversations(
        &self,
        link: &str,
    ) -> Result<Vec<mega_conversation::Model>, MegaError> {
        let model = mega_conversation::Entity::find()
            .filter(mega_conversation::Column::Link.eq(link))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn remove_mr_conversation(&self, id: i64) -> Result<(), MegaError> {
        mega_conversation::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn add_mr_conversation(
        &self,
        link: &str,
        user_id: i64,
        conv_type: ConvTypeEnum,
        comment: Option<String>,
    ) -> Result<i64, MegaError> {
        let conversation = mega_conversation::Model {
            id: generate_id(),
            link: link.to_owned(),
            user_id,
            conv_type,
            comment,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let conversation = conversation.into_active_model();
        let res = conversation.insert(self.get_connection()).await.unwrap();
        Ok(res.id)
    }
}
