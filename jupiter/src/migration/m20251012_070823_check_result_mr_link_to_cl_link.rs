use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename column `mr_link` to `cl_link` on `check_result` table
        manager
            .alter_table(
                Table::alter()
                    .table(CheckResult::Table)
                    .rename_column(CheckResultColumn::MrLink, CheckResultColumn::ClLink)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert column name change: `cl_link` back to `mr_link`
        manager
            .alter_table(
                Table::alter()
                    .table(CheckResult::Table)
                    .rename_column(CheckResultColumn::ClLink, CheckResultColumn::MrLink)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CheckResult {
    Table,
}

#[derive(DeriveIden)]
enum CheckResultColumn {
    MrLink,
    ClLink,
}
