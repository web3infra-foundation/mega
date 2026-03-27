use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop old Orion v1 tables after v2 migration is in place.
        // Order matters due to possible foreign key constraints.
        manager
            .drop_table(Table::drop().if_exists().table(Builds::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().if_exists().table(Targets::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().if_exists().table(Tasks::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: legacy v1 tables should not be recreated.
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Builds {
    Table,
}

#[derive(DeriveIden)]
enum Targets {
    Table,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
}
