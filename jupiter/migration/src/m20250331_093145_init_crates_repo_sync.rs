use extension::postgres::Type;
use sea_orm_migration::{
    prelude::*,
    schema::*,
    sea_orm::{EnumIter, Iterable},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(CrateTypeEnum)
                    .values(CrateType::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(SyncStatusEnum)
                    .values(SyncStatus::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RepoSyncResult::Table)
                    .if_not_exists()
                    .col(pk_auto(RepoSyncResult::Id))
                    .col(string(RepoSyncResult::CrateName))
                    .col(text_null(RepoSyncResult::GithubUrl))
                    .col(text(RepoSyncResult::MegaUrl))
                    .col(enumeration(
                        RepoSyncResult::Status,
                        Alias::new("sync_status_enum"),
                        SyncStatus::iter(),
                    ))
                    .col(enumeration(
                        RepoSyncResult::CrateType,
                        Alias::new("crate_type_enum"),
                        CrateType::iter(),
                    ))
                    .col(text_null(RepoSyncResult::ErrMessage))
                    .col(text(RepoSyncResult::Version))
                    .col(date_time(RepoSyncResult::CreatedAt))
                    .col(date_time(RepoSyncResult::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GithubSync::Table)
                    .if_not_exists()
                    .col(pk_auto(GithubSync::Id))
                    .col(text(GithubSync::Version))
                    .col(string(GithubSync::RepoName))
                    .col(text(GithubSync::GithubUrl))
                    .col(text(GithubSync::MegaUrl))
                    .col(date_time(GithubSync::Timestamp))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-repo_sync_result_crate_name")
                    .unique()
                    .table(RepoSyncResult::Table)
                    .col(RepoSyncResult::CrateName)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-github_sync_repo_name")
                    .table(GithubSync::Table)
                    .col(GithubSync::RepoName)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx-github_sync_repo_name").to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx-repo_sync_result_crate_name")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(GithubSync::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RepoSyncResult::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().if_exists().name(CrateTypeEnum).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().if_exists().name(SyncStatusEnum).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RepoSyncResult {
    Table,
    Id,
    CrateName,
    GithubUrl,
    MegaUrl,
    CrateType,
    Status,
    ErrMessage,
    CreatedAt,
    UpdatedAt,
    Version,
}

#[derive(DeriveIden)]
enum GithubSync {
    Table,
    Id,
    Version,
    RepoName,
    GithubUrl,
    MegaUrl,
    Timestamp,
}

#[derive(DeriveIden)]
struct CrateTypeEnum;
#[derive(Iden, EnumIter)]
enum CrateType {
    Lib,
    Application,
}

#[derive(DeriveIden)]
struct SyncStatusEnum;
#[derive(Iden, EnumIter)]
enum SyncStatus {
    Syncing,
    Succeed,
    Failed,
    Analysing,
    Analysed,
}
