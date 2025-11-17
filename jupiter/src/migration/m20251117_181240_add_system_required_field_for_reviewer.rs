use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(boolean(MegaMrReviewer::SystemRequired))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::SystemRequired)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaMrReviewer {
    #[sea_orm(iden = "mega_cl_reviewer")]
    Table,
    SystemRequired,
}
