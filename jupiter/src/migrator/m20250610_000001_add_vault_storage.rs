use sea_orm_migration::{prelude::*, schema::big_integer};

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
                    .col(big_integer(Vault::Id).primary_key().auto_increment())
                    .col(ColumnDef::new(Vault::Key).string().not_null().unique_key())
                    .col(ColumnDef::new(Vault::Value).binary().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("vault_key_unique")
                    .table(Vault::Table)
                    .col(Vault::Key)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Vault::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Vault {
    Table,
    Id,
    Key,
    Value,
}
