use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .add_column_if_not_exists(string(MegaTree::CommitId).default(""))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .add_column_if_not_exists(string(MegaBlob::CommitId).default(""))
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
enum MegaBlob {
    Table,
    CommitId,
}

#[derive(DeriveIden)]
enum MegaTree {
    Table,
    CommitId,
}
