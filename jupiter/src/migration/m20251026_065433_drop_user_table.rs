use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop legacy users table if it exists. Indexes on the table will be dropped automatically.
        manager
            .drop_table(Table::drop().if_exists().table(User::Table).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: we don't recreate the legacy users table
        let _ = manager; // silence unused warning in some compilers
        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "user")]
    Table,
}
