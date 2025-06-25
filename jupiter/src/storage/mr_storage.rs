use std::sync::Arc;

use common::model::Pagination;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use callisto::{label, mega_mr};
use callisto::sea_orm_active_enums::MergeStatusEnum;
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
        page: Pagination,
    ) -> Result<(Vec<(mega_mr::Model, Vec<label::Model>)>, u64), MegaError> {
        let paginator = mega_mr::Entity::find()
            .filter(mega_mr::Column::Status.is_in(status))
            .order_by_desc(mega_mr::Column::CreatedAt)
            .paginate(self.get_connection(), page.per_page);
        let num_pages = paginator.num_items().await?;

        let (mr_list, page) = paginator
            .fetch_page(page.page - 1)
            .await
            .map(|m| (m, num_pages))?;

        let mr_with_label: Vec<(mega_mr::Model, Vec<label::Model>)> =
            mega_mr::Entity::find()
                .filter(
                    mega_mr::Column::Id.is_in(mr_list.iter().map(|i| i.id).collect::<Vec<_>>()),
                )
                .find_with_related(label::Entity)
                .all(self.get_connection())
                .await?;
        Ok((mr_with_label, page))
    }

    pub async fn get_mr(&self, link: &str) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Link.eq(link))
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    pub async fn new_mr(
        &self,
        path: &str,
        title: &str,
        from_hash: &str,
        to_hash: &str,
    ) -> Result<String, MegaError> {
        let link = common::utils::generate_link();

        let mr = mega_mr::ActiveModel {
            id: Set(generate_id()),
            link: Set(link.clone()),
            title: Set(title.to_owned()),
            merge_date: Set(None),
            status: Set(MergeStatusEnum::Open),
            path: Set(path.to_owned()),
            from_hash: Set(from_hash.to_owned()),
            to_hash: Set(to_hash.to_owned()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };

        mr.insert(self.get_connection()).await.unwrap();
        Ok(link)
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
}
