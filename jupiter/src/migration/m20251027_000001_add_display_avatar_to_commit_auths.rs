use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // add nullable columns display_name and avatar_url
        manager
            .alter_table(
                Table::alter()
                    .table(CommitAuths::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(CommitAuths::DisplayName).string().null(),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(CommitAuths::AvatarUrl).string().null(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CommitAuths::Table)
                    .drop_column(CommitAuths::DisplayName)
                    .drop_column(CommitAuths::AvatarUrl)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CommitAuths {
    Table,
    DisplayName,
    AvatarUrl,
}
