use sea_orm_migration::{prelude::*, schema::*};

use crate::migration::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MegaMrReviewer::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaMrReviewer::Id))
                    .col(big_integer(MegaMrReviewer::MrId))
                    .col(text(MegaMrReviewer::CampsiteID))
                    .col(boolean(MegaMrReviewer::Approved))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MegaMrReviewer::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MegaMrReviewer {
    Table,
    Id,
    MrId,
    CampsiteID,
    Approved
}