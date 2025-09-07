use std::collections::HashMap;
use std::ops::Deref;

use common::model::Pagination;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, IntoActiveModel, JoinType,
    PaginatorTrait, QueryFilter, QuerySelect, RelationTrait, Set,
};

use callisto::sea_orm_active_enums::MergeStatusEnum;
use callisto::{
    check_result, item_assignees, label, mega_conversation, mega_mr, path_check_configs,
};
use common::errors::MegaError;

use crate::model::common::{ItemDetails, ListParams};
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::stg_common::combine_item_list;
use crate::storage::stg_common::query_build::{apply_sort, filter_by_assignees, filter_by_labels};

#[derive(Clone)]
pub struct MrStorage {
    pub base: BaseStorage,
}

impl Deref for MrStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl MrStorage {
    pub async fn get_open_mr_by_path(
        &self,
        path: &str,
        username: &str,
    ) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Path.eq(path))
            .filter(mega_mr::Column::Username.eq(username))
            .filter(mega_mr::Column::Status.eq(MergeStatusEnum::Open))
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn get_mr_list(
        &self,
        params: ListParams,
        page: Pagination,
    ) -> Result<(Vec<ItemDetails>, u64), MegaError> {
        let cond = Condition::all();
        let cond = filter_by_labels(cond, params.labels);
        let cond = filter_by_assignees(cond, params.assignees);

        let status = if params.status == "open" {
            vec![MergeStatusEnum::Open]
        } else if params.status == "closed" {
            vec![MergeStatusEnum::Closed, MergeStatusEnum::Merged]
        } else {
            vec![
                MergeStatusEnum::Open,
                MergeStatusEnum::Closed,
                MergeStatusEnum::Merged,
            ]
        };

        let query = mega_mr::Entity::find()
            .join(
                JoinType::LeftJoin,
                callisto::entity_ext::mega_mr::Relation::ItemLabels.def(),
            )
            .join(
                JoinType::LeftJoin,
                callisto::entity_ext::mega_mr::Relation::ItemAssignees.def(),
            )
            .filter(mega_mr::Column::Status.is_in(status))
            .filter(cond)
            .distinct();

        let mut sort_map = HashMap::new();
        sort_map.insert("created_at", mega_mr::Column::CreatedAt);
        sort_map.insert("updated_at", mega_mr::Column::UpdatedAt);

        let query = apply_sort(query, params.sort_by.as_deref(), params.asc, &sort_map);
        let paginator = query.paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;

        let (mr_list, page) = paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?;

        let ids = mr_list.iter().map(|m| m.id).collect::<Vec<_>>();

        let label_query = mega_mr::Entity::find().filter(mega_mr::Column::Id.is_in(ids.clone()));
        let label_query = apply_sort(
            label_query,
            params.sort_by.as_deref(),
            params.asc,
            &sort_map,
        );
        let labels: Vec<(mega_mr::Model, Vec<label::Model>)> = label_query
            .find_with_related(label::Entity)
            .all(self.get_connection())
            .await?;

        let assignees: Vec<(mega_mr::Model, Vec<item_assignees::Model>)> = mega_mr::Entity::find()
            .filter(mega_mr::Column::Id.is_in(ids.clone()))
            .find_with_related(item_assignees::Entity)
            .all(self.get_connection())
            .await?;

        let conversations: Vec<(mega_mr::Model, Vec<mega_conversation::Model>)> =
            mega_mr::Entity::find()
                .filter(mega_mr::Column::Id.is_in(ids))
                .find_with_related(mega_conversation::Entity)
                .all(self.get_connection())
                .await?;

        let res = combine_item_list::<mega_mr::Entity>(labels, assignees, conversations);

        Ok((res, page))
    }

    pub async fn get_mr_suggestions_by_query(
        &self,
        query: &str,
    ) -> Result<Vec<mega_mr::Model>, MegaError> {
        let keyword = format!("%{query}%");
        let res = mega_mr::Entity::find()
            .filter(
                Condition::any()
                    .add(mega_mr::Column::Link.like(&keyword))
                    .add(mega_mr::Column::Title.like(&keyword)),
            )
            .limit(5)
            .all(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn get_mr(&self, link: &str) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Link.eq(link))
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    pub async fn get_mr_labels(
        &self,
        link: &str,
    ) -> Result<Option<(mega_mr::Model, Vec<label::Model>)>, MegaError> {
        let labels: Vec<(mega_mr::Model, Vec<label::Model>)> = mega_mr::Entity::find()
            .filter(mega_mr::Column::Link.eq(link))
            .find_with_related(label::Entity)
            .all(self.get_connection())
            .await?;
        Ok(labels.first().cloned())
    }

    pub async fn get_mr_assignees(
        &self,
        link: &str,
    ) -> Result<Option<(mega_mr::Model, Vec<item_assignees::Model>)>, MegaError> {
        let assignees: Vec<(mega_mr::Model, Vec<item_assignees::Model>)> = mega_mr::Entity::find()
            .filter(mega_mr::Column::Link.eq(link))
            .find_with_related(item_assignees::Entity)
            .all(self.get_connection())
            .await?;
        Ok(assignees.first().cloned())
    }

    pub async fn new_mr(
        &self,
        path: &str,
        link: &str,
        title: &str,
        from_hash: &str,
        to_hash: &str,
        username: &str,
    ) -> Result<String, MegaError> {
        let model = mega_mr::Model::new(
            path.to_owned(),
            title.to_owned(),
            link.to_owned(),
            from_hash.to_owned(),
            to_hash.to_owned(),
            username.to_owned(),
        );
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(res.link)
    }

    pub async fn edit_title(&self, link: &str, title: &str) -> Result<(), MegaError> {
        mega_mr::Entity::update_many()
            .col_expr(mega_mr::Column::Title, Expr::value(title))
            .col_expr(
                mega_mr::Column::UpdatedAt,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .filter(mega_mr::Column::Link.eq(link))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn close_mr(&self, model: mega_mr::Model) -> Result<(), MegaError> {
        let mut a_model = model.into_active_model();
        a_model.status = Set(MergeStatusEnum::Closed);
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn reopen_mr(&self, model: mega_mr::Model) -> Result<(), MegaError> {
        let mut a_model = model.into_active_model();
        a_model.status = Set(MergeStatusEnum::Open);
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn merge_mr(&self, model: mega_mr::Model) -> Result<(), MegaError> {
        let mut a_model = model.into_active_model();
        a_model.status = Set(MergeStatusEnum::Merged);
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn update_mr_to_hash(
        &self,
        model: mega_mr::Model,
        to_hash: &str,
    ) -> Result<(), MegaError> {
        let mut a_model = model.into_active_model();
        a_model.to_hash = Set(to_hash.to_owned());
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn update_mr_hash(
        &self,
        model: mega_mr::Model,
        from_hash: &str,
        to_hash: &str,
    ) -> Result<(), MegaError> {
        let mut a_model = model.into_active_model();
        a_model.from_hash = Set(from_hash.to_owned());
        a_model.to_hash = Set(to_hash.to_owned());
        a_model.updated_at = Set(chrono::Utc::now().naive_utc());
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn get_checks_config_by_path(
        &self,
        _: &str,
    ) -> Result<Vec<path_check_configs::Model>, MegaError> {
        let models = path_check_configs::Entity::find()
            // .filter(path_check_configs::Column::Path.eq(path))
            .filter(path_check_configs::Column::Enabled.eq(true))
            .all(self.get_connection())
            .await?;
        Ok(models)
    }

    pub async fn save_check_results(
        &self,
        models: Vec<check_result::Model>,
    ) -> Result<(), MegaError> {
        let models: Vec<check_result::ActiveModel> =
            models.into_iter().map(|m| m.into_active_model()).collect();
        check_result::Entity::insert_many(models)
            .on_conflict(
                OnConflict::columns(vec![
                    check_result::Column::MrLink,
                    check_result::Column::CheckTypeCode,
                ])
                .update_columns([
                    check_result::Column::CommitId,
                    check_result::Column::Status,
                    check_result::Column::Message,
                ])
                .to_owned(),
            )
            .do_nothing()
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn get_check_result(
        &self,
        mr_link: &str,
    ) -> Result<Vec<check_result::Model>, MegaError> {
        let models = check_result::Entity::find()
            .filter(check_result::Column::MrLink.eq(mr_link))
            .all(self.get_connection())
            .await?;
        Ok(models)
    }
}
