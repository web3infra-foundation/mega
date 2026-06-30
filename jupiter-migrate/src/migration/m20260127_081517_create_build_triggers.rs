use sea_orm_migration::prelude::*;

use crate::migration::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BuildTriggers::Table)
                    .if_not_exists()
                    .col(pk_bigint(BuildTriggers::Id))
                    .col(
                        ColumnDef::new(BuildTriggers::TriggerType)
                            .string_len(50)
                            .not_null()
                            .comment("Trigger type: git_push, manual, retry, webhook, schedule"),
                    )
                    .col(
                        ColumnDef::new(BuildTriggers::TriggerSource)
                            .string_len(50)
                            .not_null()
                            .comment("Trigger source: user, system, service"),
                    )
                    .col(
                        ColumnDef::new(BuildTriggers::TriggerPayload)
                            .json_binary()
                            .not_null()
                            .comment("Trigger payload: commit, branch, tag, custom parameters"),
                    )
                    .col(
                        ColumnDef::new(BuildTriggers::TriggerTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp())
                            .comment("Trigger time"),
                    )
                    .col(
                        ColumnDef::new(BuildTriggers::TaskId)
                            .uuid()
                            .comment("Associated task ID"),
                    )
                    .col(
                        ColumnDef::new(BuildTriggers::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp())
                            .comment("Last update time"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_build_triggers_type")
                    .table(BuildTriggers::Table)
                    .col(BuildTriggers::TriggerType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_build_triggers_time")
                    .table(BuildTriggers::Table)
                    .col(BuildTriggers::TriggerTime)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_build_triggers_task_id")
                    .table(BuildTriggers::Table)
                    .col(BuildTriggers::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_build_triggers_source")
                    .table(BuildTriggers::Table)
                    .col(BuildTriggers::TriggerSource)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BuildTriggers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BuildTriggers {
    Table,
    Id,
    TriggerType,
    TriggerSource,
    TriggerPayload,
    TriggerTime,
    TaskId,
    UpdatedAt,
}
