use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tasks::Table).to_owned())
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tasks::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Tasks::ClId).big_integer().not_null())
                    .col(ColumnDef::new(Tasks::TaskName).string())
                    .col(ColumnDef::new(Tasks::Template).json_binary())
                    .col(
                        ColumnDef::new(Tasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_created_at")
                    .table(Tasks::Table)
                    .col(Tasks::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Builds::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Builds::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Builds::TaskId).uuid().not_null())
                    .col(ColumnDef::new(Builds::ExitCode).integer())
                    .col(
                        ColumnDef::new(Builds::StartAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Builds::EndAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Builds::Repo).string().not_null())
                    .col(ColumnDef::new(Builds::Target).string().not_null())
                    .col(ColumnDef::new(Builds::Args).json_binary())
                    .col(ColumnDef::new(Builds::OutputFile).string().not_null())
                    .col(
                        ColumnDef::new(Builds::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_builds_task_id")
                            .from(Builds::Table, Builds::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_builds_task_id")
                    .table(Builds::Table)
                    .col(Builds::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_builds_start_at")
                    .table(Builds::Table)
                    .col(Builds::StartAt)
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
enum Tasks {
    Table,
    Id,
    ClId,
    TaskName,
    Template,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Builds {
    Table,
    Id,
    TaskId,
    ExitCode,
    StartAt,
    EndAt,
    Repo,
    Target,
    Args,
    OutputFile,
    CreatedAt,
}
