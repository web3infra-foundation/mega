use std::collections::HashMap;
use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    JoinType, PaginatorTrait, QueryFilter, QuerySelect, RelationTrait, Set, TransactionTrait,
};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{item_assignees, item_labels, label, mega_conversation, mega_issue};
use common::errors::MegaError;
use common::model::Pagination;
use common::utils::{generate_id, generate_link};

use crate::storage::stg_common::combine_item_list;
use crate::storage::stg_common::model::{ItemDetails, LabelAssigneeParams, ListParams};
use crate::storage::stg_common::query_build::{apply_sort, filter_by_assignees, filter_by_labels};

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

    pub async fn get_issue_list(
        &self,
        params: ListParams,
        page: Pagination,
    ) -> Result<(Vec<ItemDetails>, u64), MegaError> {
        let cond = Condition::all();
        let cond = filter_by_labels(cond, params.labels);
        let cond = filter_by_author(cond, params.author);
        let cond = filter_by_assignees(cond, params.assignees);

        let query = mega_issue::Entity::find()
            .join(
                JoinType::LeftJoin,
                callisto::entity_ext::mega_issue::Relation::ItemLabels.def(),
            )
            .filter(mega_issue::Column::Status.eq(params.status))
            .filter(cond)
            .distinct();

        let mut sort_map = HashMap::new();
        sort_map.insert("created_at", mega_issue::Column::CreatedAt);
        sort_map.insert("updated_at", mega_issue::Column::UpdatedAt);

        let query = apply_sort(query, params.sort_by.as_deref(), params.asc, &sort_map);

        let paginator = query.paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;
        let (issues, page) = paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?;

        let issue_ids = issues.iter().map(|m| m.id).collect::<Vec<_>>();

        let label_query =
            mega_issue::Entity::find().filter(mega_issue::Column::Id.is_in(issue_ids.clone()));
        let label_query = apply_sort(
            label_query,
            params.sort_by.as_deref(),
            params.asc,
            &sort_map,
        );
        let labels: Vec<(mega_issue::Model, Vec<label::Model>)> = label_query
            .find_with_related(label::Entity)
            .all(self.get_connection())
            .await?;

        let assignees: Vec<(mega_issue::Model, Vec<item_assignees::Model>)> =
            mega_issue::Entity::find()
                .filter(mega_issue::Column::Id.is_in(issue_ids.clone()))
                .find_with_related(item_assignees::Entity)
                .all(self.get_connection())
                .await?;

        let conversations: Vec<(mega_issue::Model, Vec<mega_conversation::Model>)> =
            mega_issue::Entity::find()
                .filter(mega_issue::Column::Id.is_in(issue_ids))
                .find_with_related(mega_conversation::Entity)
                .all(self.get_connection())
                .await?;

        let res = combine_item_list::<mega_issue::Entity>(labels, assignees, conversations);

        Ok((res, page))
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
        username: &str,
        title: &str,
    ) -> Result<mega_issue::Model, MegaError> {
        let model = mega_issue::Model {
            id: generate_id(),
            link: generate_link(),
            title: title.to_owned(),
            author: username.to_owned(),
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

    pub async fn find_item_exist_assignees(
        &self,
        item_id: i64,
    ) -> Result<Vec<item_assignees::Model>, MegaError> {
        let item_assignees = item_assignees::Entity::find()
            .filter(item_assignees::Column::ItemId.eq(item_id))
            .all(self.get_connection())
            .await?;
        Ok(item_assignees)
    }

    pub async fn modify_labels(
        &self,
        to_add: Vec<i64>,
        to_remove: Vec<i64>,
        params: LabelAssigneeParams,
    ) -> Result<(), MegaError> {
        let txn = self.get_connection().begin().await?;

        let LabelAssigneeParams {
            item_id,
            link,
            username,
            item_type,
        } = params;

        if !to_remove.is_empty() {
            item_labels::Entity::delete_many()
                .filter(item_labels::Column::ItemId.eq(item_id))
                .filter(item_labels::Column::LabelId.is_in(to_remove.clone()))
                .exec(&txn)
                .await?;

            self.add_conversation(
                &link,
                &username,
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
                        item_type: item_type.clone(),
                    }
                    .into_active_model(),
                );
            }

            item_labels::Entity::insert_many(new_item_labels)
                .exec(&txn)
                .await?;
            self.add_conversation(
                &link,
                &username,
                Some(format!("{username} added {to_add:?}")),
                ConvTypeEnum::Label,
            )
            .await?;
        }

        txn.commit().await?;

        Ok(())
    }

    pub async fn modify_assignees(
        &self,
        to_add: Vec<String>,
        to_remove: Vec<String>,
        params: LabelAssigneeParams,
    ) -> Result<(), MegaError> {
        let txn = self.get_connection().begin().await?;

        let LabelAssigneeParams {
            item_id,
            link,
            username,
            item_type,
        } = params;

        if !to_remove.is_empty() {
            item_assignees::Entity::delete_many()
                .filter(item_assignees::Column::ItemId.eq(item_id))
                .filter(item_assignees::Column::AssignneeId.is_in(to_remove.clone()))
                .exec(&txn)
                .await?;

            self.add_conversation(
                &link,
                &username,
                Some(format!("{username} unassigned {to_remove:?}")),
                ConvTypeEnum::Assignee,
            )
            .await?;
        }

        if !to_add.is_empty() {
            let mut new_item = Vec::new();
            for assignnee_id in to_add.clone() {
                new_item.push(
                    item_assignees::Model {
                        created_at: chrono::Utc::now().naive_utc(),
                        updated_at: chrono::Utc::now().naive_utc(),
                        item_id,
                        assignnee_id,
                        item_type: item_type.clone(),
                    }
                    .into_active_model(),
                );
            }

            item_assignees::Entity::insert_many(new_item)
                .exec(&txn)
                .await?;
            self.add_conversation(
                &link,
                &username,
                Some(format!("{username} assigned {to_add:?}")),
                ConvTypeEnum::Assignee,
            )
            .await?;
        }

        txn.commit().await?;

        Ok(())
    }
}

fn filter_by_author(cond: Condition, author: Option<String>) -> Condition {
    if let Some(value) = author {
        cond.add(mega_issue::Column::Author.eq(value))
    } else {
        cond
    }
}
