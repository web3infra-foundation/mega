use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop BuildEvents
        manager
            .drop_index(Index::drop().name("idx_build_events_task_id").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(BuildEvents::Table).to_owned())
            .await?;

        // New BuildEvents
        manager
            .create_table(
                Table::create()
                    .table(BuildEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BuildEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BuildEvents::TaskId).uuid().not_null())
                    .col(ColumnDef::new(BuildEvents::RetryCount).integer().not_null())
                    .col(ColumnDef::new(BuildEvents::ExitCode).integer().null())
                    .col(ColumnDef::new(BuildEvents::Log).string().null())
                    .col(
                        ColumnDef::new(BuildEvents::LogOutputFile)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuildEvents::StartAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(BuildEvents::EndAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(BuildEvents::Table, BuildEvents::TaskId)
                            .to(OrionTasks::Table, OrionTasks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes on foreign key columns for better join performance
        manager
            .create_index(
                Index::create()
                    .name("idx_build_events_task_id")
                    .table(BuildEvents::Table)
                    .col(BuildEvents::TaskId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        // Do not anything.
        Ok(())
    }
}

#[derive(DeriveIden)]
enum BuildEvents {
    Table,
    Id,
    TaskId,
    RetryCount,
    ExitCode,
    Log,
    LogOutputFile,
    StartAt,
    EndAt,
}

#[derive(DeriveIden)]
enum OrionTasks {
    Table,
    Id,
}
