use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop Builds, BuildEvents, Tasks, Targets
        manager
            .drop_table(Table::drop().table(Builds::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(BuildEvents::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Targets::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tasks::Table).if_exists().to_owned())
            .await?;

        // Create Tasks
        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tasks::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Tasks::Changes).json_binary().not_null())
                    .col(ColumnDef::new(Tasks::RepoName).string().not_null())
                    .col(
                        ColumnDef::new(Tasks::CreatedAt)
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
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Targets
        manager
            .create_table(
                Table::create()
                    .table(Targets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Targets::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Targets::TaskId).uuid().not_null())
                    .col(ColumnDef::new(Targets::Path).json_binary().not_null())
                    .col(ColumnDef::new(Targets::TargetState).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Targets::Table, Targets::TaskId)
                            .to(Tasks::Table, Tasks::Id)
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
                    .name("idx_targets_task_id")
                    .table(Targets::Table)
                    .col(Targets::TaskId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();
    }
}

#[derive(DeriveIden)]
enum Builds {
    Table,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
    Changes,
    RepoName,
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
enum Targets {
    Table,
    Id,
    TaskId,
    Path,
    TargetState,
}
