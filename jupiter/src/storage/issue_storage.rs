use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_conversation, mega_issue};
use common::errors::MegaError;
use common::model::Pagination;
use common::utils::{generate_id, generate_link};

#[derive(Clone)]
pub struct IssueStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl IssueStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        IssueStorage { connection }
    }

    pub fn mock() -> Self {
        IssueStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn get_issue_by_status(
        &self,
        status: &str,
        page: Pagination,
    ) -> Result<(Vec<mega_issue::Model>, u64), MegaError> {
        let paginator = mega_issue::Entity::find()
            .filter(mega_issue::Column::Status.eq(status))
            .order_by_desc(mega_issue::Column::CreatedAt)
            .paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;
        Ok(paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?)
    }

    pub async fn get_issue(&self, link: &str) -> Result<Option<mega_issue::Model>, MegaError> {
        let model = mega_issue::Entity::find()
            .filter(mega_issue::Column::Link.eq(link))
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn save_issue(
        &self,
        user_id: i64,
        title: &str,
    ) -> Result<mega_issue::Model, MegaError> {
        let model = mega_issue::Model {
            id: generate_id(),
            link: generate_link(),
            title: title.to_owned(),
            owner: user_id,
            status: "open".to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            closed_at: None,
        };
        model
            .clone()
            .into_active_model()
            .insert(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn close_issue(&self, link: &str) -> Result<(), MegaError> {
        if let Some(model) = self.get_issue(link).await.unwrap() {
            let mut issue = model.into_active_model();
            issue.status = Set("closed".to_owned());
            issue.update(self.get_connection()).await.unwrap();
        };
        Ok(())
    }

    pub async fn reopen_issue(&self, link: &str) -> Result<(), MegaError> {
        if let Some(model) = self.get_issue(link).await.unwrap() {
            let mut issue = model.into_active_model();
            issue.status = Set("open".to_owned());
            issue.update(self.get_connection()).await.unwrap();
        };
        Ok(())
    }

    pub async fn get_issue_conversations(
        &self,
        link: &str,
    ) -> Result<Vec<mega_conversation::Model>, MegaError> {
        let model = mega_conversation::Entity::find()
            .filter(mega_conversation::Column::Link.eq(link))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn remove_issue_conversation(&self, id: i64) -> Result<(), MegaError> {
        mega_conversation::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn add_issue_conversation(
        &self,
        link: &str,
        user_id: i64,
        comment: Option<String>,
    ) -> Result<i64, MegaError> {
        let conversation = mega_conversation::Model {
            id: generate_id(),
            link: link.to_owned(),
            user_id,
            conv_type: ConvTypeEnum::Comment,
            comment,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let conversation = conversation.into_active_model();
        let res = conversation.insert(self.get_connection()).await.unwrap();
        Ok(res.id)
    }
}
