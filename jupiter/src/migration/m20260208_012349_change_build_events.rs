use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename index to retry_count
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .rename_column(BuildEvents::Index, BuildEvents::RetryCount)
                    .to_owned(),
            )
            .await?;

        // Add log
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .add_column(ColumnDef::new(BuildEvents::Log).string().null())
                    .to_owned(),
            )
            .await?;

        // Change start_at to not null
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .modify_column(ColumnDef::new(BuildEvents::StartAt).not_null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .rename_column(BuildEvents::RetryCount, BuildEvents::Index)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .drop_column(BuildEvents::Log)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .modify_column(ColumnDef::new(BuildEvents::StartAt).null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum BuildEvents {
    Table,
    Index,      // Need be renamed
    RetryCount, // Need rename
    Log,        // Need add
    StartAt,    // Need change not null
}
