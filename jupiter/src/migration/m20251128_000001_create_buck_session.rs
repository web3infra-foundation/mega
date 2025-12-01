use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create buck_session table
        manager
            .create_table(
                Table::create()
                    .table(BuckSession::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BuckSession::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BuckSession::SessionId)
                            .string_len(8)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(BuckSession::UserId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(BuckSession::RepoPath).text().not_null())
                    .col(
                        ColumnDef::new(BuckSession::Status)
                            .string_len(20)
                            .not_null()
                            .default("created"),
                    )
                    .col(ColumnDef::new(BuckSession::CommitMessage).text())
                    .col(ColumnDef::new(BuckSession::FromHash).string_len(40))
                    .col(
                        ColumnDef::new(BuckSession::ExpiresAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuckSession::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuckSession::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for buck_session
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_buck_session_user")
                    .table(BuckSession::Table)
                    .col(BuckSession::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_buck_session_status")
                    .table(BuckSession::Table)
                    .col(BuckSession::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_buck_session_expires")
                    .table(BuckSession::Table)
                    .col(BuckSession::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        // Create buck_session_file table
        manager
            .create_table(
                Table::create()
                    .table(BuckSessionFile::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BuckSessionFile::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BuckSessionFile::SessionId)
                            .string_len(8)
                            .not_null(),
                    )
                    .col(ColumnDef::new(BuckSessionFile::FilePath).text().not_null())
                    .col(
                        ColumnDef::new(BuckSessionFile::FileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuckSessionFile::FileHash)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BuckSessionFile::FileMode)
                            .string_len(10)
                            .default("100644"),
                    )
                    .col(
                        ColumnDef::new(BuckSessionFile::UploadStatus)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(BuckSessionFile::UploadReason).string_len(20))
                    .col(ColumnDef::new(BuckSessionFile::BlobId).string_len(40))
                    .col(ColumnDef::new(BuckSessionFile::UploadedAt).timestamp())
                    .col(
                        ColumnDef::new(BuckSessionFile::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session")
                            .from(BuckSessionFile::Table, BuckSessionFile::SessionId)
                            .to(BuckSession::Table, BuckSession::SessionId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for buck_session_file
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_session_file_session")
                    .table(BuckSessionFile::Table)
                    .col(BuckSessionFile::SessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_session_file_status")
                    .table(BuckSessionFile::Table)
                    .col(BuckSessionFile::SessionId)
                    .col(BuckSessionFile::UploadStatus)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for (session_id, file_path)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uk_session_file")
                    .table(BuckSessionFile::Table)
                    .col(BuckSessionFile::SessionId)
                    .col(BuckSessionFile::FilePath)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order (child first)
        manager
            .drop_table(Table::drop().table(BuckSessionFile::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(BuckSession::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum BuckSession {
    Table,
    Id,
    SessionId,
    UserId,
    RepoPath,
    Status,
    CommitMessage,
    FromHash,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum BuckSessionFile {
    Table,
    Id,
    SessionId,
    FilePath,
    FileSize,
    FileHash,
    FileMode,
    UploadStatus,
    UploadReason,
    BlobId,
    UploadedAt,
    CreatedAt,
}
