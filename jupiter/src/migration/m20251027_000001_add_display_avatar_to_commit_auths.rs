use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // add nullable columns display_name and avatar_url
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                // Sqlite does not support multiple alter options in a single statement
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .add_column_if_not_exists(
                                ColumnDef::new(CommitAuths::DisplayName).string().null(),
                            )
                            .to_owned(),
                    )
                    .await?;
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .add_column_if_not_exists(
                                ColumnDef::new(CommitAuths::AvatarUrl).string().null(),
                            )
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
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
            }
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .drop_column(CommitAuths::DisplayName)
                            .to_owned(),
                    )
                    .await?;
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .drop_column(CommitAuths::AvatarUrl)
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .drop_column(CommitAuths::DisplayName)
                            .drop_column(CommitAuths::AvatarUrl)
                            .to_owned(),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CommitAuths {
    Table,
    DisplayName,
    AvatarUrl,
}
