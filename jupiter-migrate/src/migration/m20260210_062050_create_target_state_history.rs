use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // create target state histories table
        manager
            .create_table(
                Table::create()
                    .table(TargetStateHistories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TargetStateHistories::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TargetStateHistories::BuildTargetId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TargetStateHistories::BuildEventId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TargetStateHistories::TargetState)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TargetStateHistories::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                TargetStateHistories::Table,
                                TargetStateHistories::BuildEventId,
                            )
                            .to(BuildEvents::Table, BuildEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                TargetStateHistories::Table,
                                TargetStateHistories::BuildTargetId,
                            )
                            .to(BuildTargets::Table, BuildTargets::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_target_state_histories_build_target_id")
                    .table(TargetStateHistories::Table)
                    .col(TargetStateHistories::BuildTargetId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_target_state_histories_build_event_id")
                    .table(TargetStateHistories::Table)
                    .col(TargetStateHistories::BuildEventId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_target_state_histories_build_target_id")
                    .table(TargetStateHistories::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_target_state_histories_build_event_id")
                    .table(TargetStateHistories::Table)
                    .to_owned(),
            )
            .await?;

        // drop target state histories table
        manager
            .drop_table(Table::drop().table(TargetStateHistories::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum TargetStateHistories {
    Table,
    Id,
    BuildTargetId,
    BuildEventId,
    TargetState,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BuildEvents {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BuildTargets {
    Table,
    Id,
}
