use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_cl_path").to_owned())
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cl_path_link")
                    .unique()
                    .table(MegaCl::Table)
                    .col(MegaCl::Path)
                    .col(MegaCl::Link)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaCl {
    Table,
    Link,
    Path,
}
