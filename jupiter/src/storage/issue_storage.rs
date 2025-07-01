use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{item_labels, label, mega_conversation, mega_issue};
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
    ) -> Result<(Vec<(mega_issue::Model, Vec<label::Model>)>, u64), MegaError> {
        let paginator = mega_issue::Entity::find()
            .filter(mega_issue::Column::Status.eq(status))
            .order_by_desc(mega_issue::Column::CreatedAt)
            .paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;
        let (issues, page) = paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?;

        let issues_with_label: Vec<(mega_issue::Model, Vec<label::Model>)> =
            mega_issue::Entity::find()
                .filter(
                    mega_issue::Column::Id.is_in(issues.iter().map(|i| i.id).collect::<Vec<_>>()),
                )
                .order_by_desc(mega_issue::Column::CreatedAt)
                .find_with_related(label::Entity)
                .all(self.get_connection())
                .await?;

        Ok((issues_with_label, page))
    }

    pub async fn get_issue(&self, link: &str) -> Result<Option<mega_issue::Model>, MegaError> {
        let model = mega_issue::Entity::find()
            .filter(mega_issue::Column::Link.eq(link))
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn get_issue_by_id(&self, id: i64) -> Result<Option<mega_issue::Model>, MegaError> {
        let model = mega_issue::Entity::find_by_id(id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn save_issue(
        &self,
        user_id: &str,
        title: &str,
    ) -> Result<mega_issue::Model, MegaError> {
        let model = mega_issue::Model {
            id: generate_id(),
            link: generate_link(),
            title: title.to_owned(),
            user_id: user_id.to_owned(),
            status: "open".to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            closed_at: None,
        };
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(res)
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

    pub async fn get_conversations(
        &self,
        link: &str,
    ) -> Result<Vec<mega_conversation::Model>, MegaError> {
        let model = mega_conversation::Entity::find()
            .filter(mega_conversation::Column::Link.eq(link))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn remove_conversation(&self, id: i64) -> Result<(), MegaError> {
        mega_conversation::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn add_conversation(
        &self,
        link: &str,
        username: &str,
        comment: Option<String>,
        conv_type: ConvTypeEnum,
    ) -> Result<i64, MegaError> {
        let conversation = mega_conversation::Model {
            id: generate_id(),
            link: link.to_owned(),
            conv_type,
            comment,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            username: username.to_owned(),
        };
        let conversation = conversation.into_active_model();
        let res = conversation.insert(self.get_connection()).await.unwrap();
        Ok(res.id)
    }

    pub async fn new_label(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> Result<label::Model, MegaError> {
        let model = label::Model {
            id: generate_id(),
            name: name.to_owned(),
            color: color.to_owned(),
            description: description.to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn list_labels_by_page(
        &self,
        page: Pagination,
    ) -> Result<(Vec<label::Model>, u64), MegaError> {
        let paginator = label::Entity::find().paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;
        Ok(paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?)
    }

    pub async fn find_item_exist_labels(
        &self,
        item_id: i64,
    ) -> Result<Vec<item_labels::Model>, MegaError> {
        let item_labels = item_labels::Entity::find()
            .filter(item_labels::Column::ItemId.eq(item_id))
            .all(self.get_connection())
            .await?;
        Ok(item_labels)
    }

    pub async fn modify_labels(
        &self,
        username: &str,
        item_id: i64,
        link: &str,
        to_add: Vec<i64>,
        to_remove: Vec<i64>,
    ) -> Result<(), MegaError> {
        let txn = self.get_connection().begin().await?;

        if !to_remove.is_empty() {
            item_labels::Entity::delete_many()
                .filter(item_labels::Column::ItemId.eq(item_id))
                .filter(item_labels::Column::LabelId.is_in(to_remove.clone()))
                .exec(&txn)
                .await?;

            self.add_conversation(
                link,
                username,
                Some(format!("{username} removed {to_remove:?}")),
                ConvTypeEnum::Label,
            )
            .await?;
        }

        if !to_add.is_empty() {
            let mut new_item_labels = Vec::new();
            for label_id in to_add.clone() {
                new_item_labels.push(
                    item_labels::Model {
                        created_at: chrono::Utc::now().naive_utc(),
                        updated_at: chrono::Utc::now().naive_utc(),
                        item_id,
                        label_id,
                        item_type: String::from("issue"),
                    }
                    .into_active_model(),
                );
            }

            item_labels::Entity::insert_many(new_item_labels)
                .exec(&txn)
                .await?;
            self.add_conversation(
                link,
                username,
                Some(format!("{username} added {to_add:?}")),
                ConvTypeEnum::Label,
            )
            .await?;
        }

        txn.commit().await?;

        Ok(())
    }
}
