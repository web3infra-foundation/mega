use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // rename table issue_mr_references -> issue_cl_references
        manager
            .rename_table(
                Table::rename()
                    .table(IssueMrReferences::Table, IssueClReferences::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // reverse rename: issue_cl_references -> issue_mr_references
        manager
            .rename_table(
                Table::rename()
                    .table(IssueClReferences::Table, IssueMrReferences::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum IssueMrReferences {
    Table,
}

#[derive(DeriveIden)]
enum IssueClReferences {
    Table,
}
