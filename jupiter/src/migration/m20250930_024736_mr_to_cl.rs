use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                Table::rename()
                    .table(MegaMr::Table, MegaCl::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .rename_table(
                Table::rename()
                    .table(MegaMrReviewer::Table, MegaClReviewer::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .rename_column(MegaClReviewerColumn::MrLink, MegaClReviewerColumn::ClLink)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .rename_column(MegaClReviewerColumn::ClLink, MegaClReviewerColumn::MrLink)
                    .to_owned(),
            )
            .await?;
        manager
            .rename_table(
                Table::rename()
                    .table(MegaCl::Table, MegaMr::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .rename_table(
                Table::rename()
                    .table(MegaClReviewer::Table, MegaMrReviewer::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaMr {
    Table,
}
#[derive(DeriveIden)]
enum MegaCl {
    Table,
}
#[derive(DeriveIden)]
enum MegaMrReviewer {
    Table,
}
#[derive(DeriveIden)]
enum MegaClReviewer {
    Table,
}
#[derive(DeriveIden)]
enum MegaClReviewerColumn {
    MrLink,
    ClLink,
}