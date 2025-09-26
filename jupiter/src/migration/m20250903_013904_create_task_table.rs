use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Builds::Table).to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::TaskId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tasks::BuildIds).json_binary().not_null())
                    .col(ColumnDef::new(Tasks::OutputFiles).json_binary().not_null())
                    .col(ColumnDef::new(Tasks::ExitCode).integer())
                    .col(
                        ColumnDef::new(Tasks::StartAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tasks::EndAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Tasks::RepoName).string().not_null())
                    .col(ColumnDef::new(Tasks::Target).string().not_null())
                    .col(ColumnDef::new(Tasks::Arguments).string().not_null())
                    .col(ColumnDef::new(Tasks::Cl).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_cl")
                    .table(Tasks::Table)
                    .col(Tasks::Cl)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_start_at")
                    .table(Tasks::Table)
                    .col(Tasks::StartAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Builds {
    Table,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    TaskId,
    BuildIds,
    OutputFiles,
    ExitCode,
    StartAt,
    EndAt,
    RepoName,
    Target,
    Arguments,
    Cl,
}
