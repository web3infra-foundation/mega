use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Split each alter into individual statements for SQLite compatibility

        // --- mega_cl_reviewer ---
        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaClReviewer::Username)
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
                    .table(MegaClReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaClReviewer::ClLink)
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
                    .table(MegaClReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaClReviewer::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaClReviewer::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::CampsiteID)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::ClId)
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

        // Reverse mega_cl_reviewer changes
        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::Username)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::ClLink)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaClReviewer::Table)
                    .drop_column(MegaClReviewer::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
#[sea_orm(iden = "mega_cl_reviewer")]
enum MegaClReviewer {
    Table,
    ClId,
    ClLink,
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
