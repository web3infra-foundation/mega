use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_dynamic_sidebar_order_index")
                    .table(DynamicSidebar::Table)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("idx_dynamic_sidebar_order_index")
                    .table(DynamicSidebar::Table)
                    .col(DynamicSidebar::OrderIndex)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum DynamicSidebar {
    Table,
    OrderIndex,
}
