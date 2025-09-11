use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::Username)
                            .string()
                            .not_null()
                            .unique_key()
                            .default("".to_string()),
                    )
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::MrLink)
                            .string()
                            .not_null()
                            .default("".to_string()),
                    )
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .add_column(
                        ColumnDef::new(MegaMrReviewer::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .drop_column(MegaMrReviewer::CampsiteID)
                    .drop_column(MegaMrReviewer::MrId)
                    .to_owned(),
            )
            .await?;

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
        manager
            .alter_table(
                Table::alter()
                    .table(MegaMrReviewer::Table)
                    .drop_column(MegaMrReviewer::Username)
                    .to_owned(),
            )
            .await
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
