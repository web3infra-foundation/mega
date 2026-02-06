use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create TasksRefactor
        manager
            .create_table(
                Table::create()
                    .table(TasksRefactor::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TasksRefactor::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TasksRefactor::Changes)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TasksRefactor::RepoName).string().not_null())
                    .col(ColumnDef::new(TasksRefactor::CL).string().not_null())
                    .col(
                        ColumnDef::new(TasksRefactor::CreatedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create BuildEventsRefactor
        manager
            .create_table(
                Table::create()
                    .table(BuildEventsRefactor::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BuildEventsRefactor::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::Index)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::TaskId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::ExitCode)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::LogOutputFile)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::StartAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(BuildEventsRefactor::EndAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(BuildEventsRefactor::Table, BuildEventsRefactor::TaskId)
                            .to(TasksRefactor::Table, TasksRefactor::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create TargetsRefactor
        manager
            .create_table(
                Table::create()
                    .table(TargetsRefactor::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TargetsRefactor::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TargetsRefactor::TaskId).uuid().not_null())
                    .col(
                        ColumnDef::new(TargetsRefactor::Path)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TargetsRefactor::TargetState)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TargetsRefactor::Table, TargetsRefactor::TaskId)
                            .to(TasksRefactor::Table, TasksRefactor::Id)
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
                    .name("idx_build_events_refactor_task_id")
                    .table(BuildEventsRefactor::Table)
                    .col(BuildEventsRefactor::TaskId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_targets_refactor_task_id")
                    .table(TargetsRefactor::Table)
                    .col(TargetsRefactor::TaskId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes created in `up`
        manager
            .drop_index(
                Index::drop()
                    .name("idx_targets_refactor_task_id")
                    .table(TargetsRefactor::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_build_events_refactor_task_id")
                    .table(BuildEventsRefactor::Table)
                    .to_owned(),
            )
            .await?;
        // Drop tables created in `up` (children before parent)
        manager
            .drop_table(Table::drop().table(TargetsRefactor::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(BuildEventsRefactor::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TasksRefactor::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum TasksRefactor {
    Table,
    Id,
    Changes,
    RepoName,
    CL,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BuildEventsRefactor {
    Table,
    Index,
    Id,
    TaskId,
    ExitCode,
    LogOutputFile,
    StartAt,
    EndAt,
}

#[derive(DeriveIden)]
enum TargetsRefactor {
    Table,
    Id,
    TaskId,
    Path,
    TargetState,
}
