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
                    .table(GpgKey::Table)
                    .if_not_exists()
                    .col(pk_bigint(GpgKey::Id))
                    .col(text(GpgKey::UserId))
                    .col(text(GpgKey::KeyId))
                    .col(text(GpgKey::PublicKey))
                    .col(text(GpgKey::Fingerprint).unique_key())
                    .col(text(GpgKey::Alias))
                    .col(timestamp(GpgKey::CreatedAt))
                    .col(timestamp_null(GpgKey::ExpiresAt))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GpgKey::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GpgKey {
    Table,
    Id,
    KeyId,
    UserId,
    PublicKey,
    Fingerprint,
    Alias,
    CreatedAt,
    ExpiresAt,
}
