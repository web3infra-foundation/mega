use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) Rename column `mr_link` -> `cl_link` on `check_result`
        manager
            .alter_table(
                Table::alter()
                    .table(CheckResult::Table)
                    .rename_column(CheckResultColumn::MrLink, CheckResultColumn::ClLink)
                    .to_owned(),
            )
            .await?;

        // 2) Rename table `issue_mr_references` -> `issue_cl_references`
        manager
            .rename_table(
                Table::rename()
                    .table(IssueMrReferences::Table, IssueClReferences::Table)
                    .to_owned(),
            )
            .await?;

        // 3) Rename column `is_mr` -> `is_cl` on `mega_refs`
        manager
            .alter_table(
                Table::alter()
                    .table(MegaRefs::Table)
                    .rename_column(MegaRefsColumn::IsMr, MegaRefsColumn::IsCl)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // reverse in opposite order
        manager
            .alter_table(
                Table::alter()
                    .table(MegaRefs::Table)
                    .rename_column(MegaRefsColumn::IsCl, MegaRefsColumn::IsMr)
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(
                Table::rename()
                    .table(IssueClReferences::Table, IssueMrReferences::Table)
                    .to_owned(),
            )
            .await?;

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

// Iden enums used for the three operations
#[derive(DeriveIden)]
enum CheckResult {
    Table,
}

#[derive(DeriveIden)]
enum CheckResultColumn {
    MrLink,
    ClLink,
}

#[derive(DeriveIden)]
enum IssueMrReferences {
    Table,
}

#[derive(DeriveIden)]
enum IssueClReferences {
    Table,
}

#[derive(DeriveIden)]
enum MegaRefs {
    Table,
}

#[derive(DeriveIden)]
enum MegaRefsColumn {
    IsMr,
    IsCl,
}
