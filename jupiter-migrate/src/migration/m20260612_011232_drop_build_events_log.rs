use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The `log` column was never populated (logs live in the artifact store
        // referenced by `log_output_file`); drop it to remove the dead field.
        manager
            .alter_table(
                Table::alter()
                    .table(BuildEvents::Table)
                    .drop_column(BuildEvents::Log)
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
                    .add_column(ColumnDef::new(BuildEvents::Log).string().null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum BuildEvents {
    Table,
    Log,
}
