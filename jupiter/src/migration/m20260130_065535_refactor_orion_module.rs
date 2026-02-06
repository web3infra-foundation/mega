use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create OrionTasks
        manager
            .create_table(
                Table::create()
                    .table(OrionTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrionTasks::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OrionTasks::Changes).json_binary().not_null())
                    .col(ColumnDef::new(OrionTasks::RepoName).string().not_null())
                    .col(ColumnDef::new(OrionTasks::CL).string().not_null())
                    .col(
                        ColumnDef::new(OrionTasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create BuildEvents
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
                    .col(ColumnDef::new(BuildEvents::Index).integer().not_null())
                    .col(ColumnDef::new(BuildEvents::TaskId).uuid().not_null())
                    .col(ColumnDef::new(BuildEvents::ExitCode).integer().null())
                    .col(
                        ColumnDef::new(BuildEvents::LogOutputFile)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuildEvents::StartAt)
                            .timestamp_with_time_zone()
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

        // Create BuildTargets
        manager
            .create_table(
                Table::create()
                    .table(BuildTargets::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BuildTargets::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BuildTargets::TaskId).uuid().not_null())
                    .col(ColumnDef::new(BuildTargets::Path).json_binary().not_null())
                    .col(ColumnDef::new(BuildTargets::TargetState).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(BuildTargets::Table, BuildTargets::TaskId)
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
        manager
            .create_index(
                Index::create()
                    .name("idx_build_targets_task_id")
                    .table(BuildTargets::Table)
                    .col(BuildTargets::TaskId)
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
                    .name("idx_build_targets_task_id")
                    .table(BuildTargets::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_build_events_task_id")
                    .table(BuildEvents::Table)
                    .to_owned(),
            )
            .await?;
        // Drop tables created in `up` (children before parent)
        manager
            .drop_table(Table::drop().table(BuildTargets::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(BuildEvents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OrionTasks::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum OrionTasks {
    Table,
    Id,
    Changes,
    RepoName,
    CL,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BuildEvents {
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
enum BuildTargets {
    Table,
    Id,
    TaskId,
    Path,
    TargetState,
}
