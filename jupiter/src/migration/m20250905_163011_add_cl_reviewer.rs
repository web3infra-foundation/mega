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
                    .table(MegaClReviewer::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaClReviewer::Id))
                    .col(big_integer(MegaClReviewer::ClId))
                    .col(text(MegaClReviewer::CampsiteID))
                    .col(boolean(MegaClReviewer::Approved))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MegaClReviewer::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MegaClReviewer {
    Table,
    Id,
    ClId,
    CampsiteID,
    Approved,
}
