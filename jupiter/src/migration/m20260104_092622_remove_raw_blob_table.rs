use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// It will drop the specified table from the database.
    /// The table will be dropped with idempotency, ensuring no error if the table doesn't exist.
    ///
    /// Purpose: Drop outdated or unused table `raw_blob` to clean up the database.
    /// Reason: The table is no longer in use and should be removed to maintain a cleaner and more efficient database.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop table raw_blob if it exists
        manager
            .drop_table(Table::drop().table(RawBlob::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum RawBlob {
    #[sea_orm(iden = "raw_blob")]
    Table,
}
