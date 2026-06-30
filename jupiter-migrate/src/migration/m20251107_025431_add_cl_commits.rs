use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(MegaClCommits::Table)
                    .if_not_exists()
                    .col(string(MegaClCommits::ClLink))
                    .col(string(MegaClCommits::CommitSha))
                    .col(string(MegaClCommits::AuthorName))
                    .col(string(MegaClCommits::AuthorEmail))
                    .col(text(MegaClCommits::Message))
                    .primary_key(
                        Index::create()
                            .col(MegaClCommits::ClLink)
                            .col(MegaClCommits::CommitSha),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .drop_column(MegaTree::CommitId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .drop_column(MegaBlob::CommitId)
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
enum MegaClCommits {
    Table,
    ClLink,
    CommitSha,
    AuthorName,
    AuthorEmail,
    Message,
}

#[derive(DeriveIden)]
enum MegaBlob {
    Table,
    CommitId,
}

#[derive(DeriveIden)]
enum MegaTree {
    Table,
    CommitId,
}
