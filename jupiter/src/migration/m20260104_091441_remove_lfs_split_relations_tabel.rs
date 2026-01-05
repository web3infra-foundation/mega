use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// It will drop the specified table from the database.
    /// The table will be dropped with idempotency, ensuring no error if the table doesn't exist.
    ///
    /// Purpose: Drop outdated or unused table `lfs_split_relations` to clean up the database.
    /// Reason: The table is no longer in use and should be removed to maintain a cleaner and more efficient database.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop table lfs_split_relations if it exists
        manager
            .drop_table(Table::drop().table(LfsSplitRelations::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum LfsSplitRelations {
    #[sea_orm(iden = "lfs_split_relations")]
    Table,
}
