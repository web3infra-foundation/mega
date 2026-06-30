use sea_orm_migration::{
    prelude::*,
    sea_orm::DatabaseBackend,
    sea_query::{InsertStatement, Query},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop legacy users table if it exists. Indexes on the table will be dropped automatically.
        manager
            .drop_table(Table::drop().if_exists().table(User::Table).to_owned())
            .await?;

        // Also drop redundant column `author_email` from commit_auths
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                // SQLite doesn't support DROP COLUMN directly; perform rename-copy-drop
                // 1) Rename old table to a temporary name
                manager
                    .exec_stmt(
                        Table::rename()
                            .table(CommitAuths::Table, CommitAuthsOld::Table)
                            .to_owned(),
                    )
                    .await?;

                // 2) Create the new commit_auths table without author_email
                manager
                    .create_table(
                        Table::create()
                            .table(CommitAuths::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(CommitAuths::Id)
                                    .string()
                                    .not_null()
                                    .primary_key(),
                            )
                            .col(ColumnDef::new(CommitAuths::CommitSha).string().not_null())
                            .col(ColumnDef::new(CommitAuths::MatchedUsername).string().null())
                            .col(
                                ColumnDef::new(CommitAuths::IsAnonymous)
                                    .boolean()
                                    .not_null(),
                            )
                            .col(ColumnDef::new(CommitAuths::MatchedAt).timestamp().null())
                            .col(
                                ColumnDef::new(CommitAuths::CreatedAt)
                                    .timestamp()
                                    .not_null(),
                            )
                            .to_owned(),
                    )
                    .await?;

                // 3) Copy data from old to new (excluding author_email)
                let select = Query::select()
                    .columns([
                        CommitAuthsOld::Id,
                        CommitAuthsOld::CommitSha,
                        CommitAuthsOld::MatchedUsername,
                        CommitAuthsOld::IsAnonymous,
                        CommitAuthsOld::MatchedAt,
                        CommitAuthsOld::CreatedAt,
                    ])
                    .from(CommitAuthsOld::Table)
                    .to_owned();

                let mut insert: InsertStatement = InsertStatement::new();
                insert.into_table(CommitAuths::Table);
                insert.columns([
                    CommitAuths::Id,
                    CommitAuths::CommitSha,
                    CommitAuths::MatchedUsername,
                    CommitAuths::IsAnonymous,
                    CommitAuths::MatchedAt,
                    CommitAuths::CreatedAt,
                ]);
                let insert = insert
                    .select_from(select)
                    .map_err(|e| DbErr::Custom(e.to_string()))?
                    .to_owned();

                manager.exec_stmt(insert).await?;

                // 4) Drop the old table
                manager
                    .drop_table(Table::drop().table(CommitAuthsOld::Table).to_owned())
                    .await?;

                Ok(())
            }
            // Postgres/MySQL support DROP COLUMN
            _ => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(CommitAuths::Table)
                            .drop_column(CommitAuths::AuthorEmail)
                            .to_owned(),
                    )
                    .await
            }
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: we don't recreate the legacy users table
        let _ = manager; // silence unused warning in some compilers
        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "user")]
    Table,
}

#[derive(DeriveIden)]
enum CommitAuths {
    #[sea_orm(iden = "commit_auths")]
    Table,
    Id,
    CommitSha,
    AuthorEmail,
    MatchedUsername,
    IsAnonymous,
    MatchedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum CommitAuthsOld {
    #[sea_orm(iden = "commit_auths_old")]
    Table,
    Id,
    CommitSha,
    MatchedUsername,
    IsAnonymous,
    MatchedAt,
    CreatedAt,
}
