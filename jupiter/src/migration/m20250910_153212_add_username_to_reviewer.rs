use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Split each alter into individual statements for SQLite compatibility

        // --- mega_mr_reviewer ---
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::Username)
                            .string()
                            .not_null()
                            .default("".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::MrLink)
                            .string()
                            .not_null()
                            .default("".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::CampsiteID)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::MrId)
                    .to_owned(),
            )
            .await?;

        // --- mega_conversation ---
        manager
            .alter_table(
                Table::alter()
                    .table(MegaConversation::Table)
                    .add_column(ColumnDef::new(MegaConversation::Resolved).boolean().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverse the Resolved column
        manager
            .alter_table(
                Table::alter()
                    .table(MegaConversation::Table)
                    .drop_column(MegaConversation::Resolved)
                    .to_owned(),
            )
            .await?;

        // Reverse mega_mr_reviewer changes
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::Username)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::MrLink)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
#[sea_orm(iden = "mega_mr_reviewer")]
enum MegaMrReviewer {
    Table,
    MrId,
    MrLink,
    CampsiteID,
    Username,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
#[sea_orm(iden = "mega_conversation")]
enum MegaConversation {
    Table,
    Resolved,
}
