use sea_orm_migration::{
    prelude::*,
    schema::{binary, string},
};

use crate::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Vault::Table)
                    .if_not_exists()
                    .col(pk_bigint(Vault::Id))
                    .col(string(Vault::Key))
                    .col(binary(Vault::Value))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Vault {
    Table,
    Id,
    Key,
    Value,
}
