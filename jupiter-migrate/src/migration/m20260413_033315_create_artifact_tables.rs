//! Artifact protocol storage (`docs/artifacts-protocol.md` §10).
//!
//! Tables: `artifact_objects`, `artifact_sets`, `artifact_set_files`.

use sea_orm_migration::{prelude::*, schema::*};

use crate::migration::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArtifactObjects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtifactObjects::Oid)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(big_integer(ArtifactObjects::SizeBytes))
                    .col(text_null(ArtifactObjects::ContentType))
                    .col(text(ArtifactObjects::StorageKey))
                    .col(date_time(ArtifactObjects::CreatedAt))
                    .col(date_time(ArtifactObjects::LastSeenAt))
                    .col(ColumnDef::new(ArtifactObjects::Integrity).json().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_artifact_objects_last_seen_at")
                    .table(ArtifactObjects::Table)
                    .col(ArtifactObjects::LastSeenAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ArtifactSets::Table)
                    .if_not_exists()
                    .col(pk_bigint(ArtifactSets::Id))
                    .col(string(ArtifactSets::Repo))
                    .col(string(ArtifactSets::Namespace))
                    .col(string(ArtifactSets::ObjectType))
                    .col(string(ArtifactSets::ArtifactSetId))
                    .col(ColumnDef::new(ArtifactSets::Metadata).json().null())
                    .col(string_null(ArtifactSets::CreatedBy))
                    .col(date_time(ArtifactSets::CreatedAt))
                    .col(date_time_null(ArtifactSets::ExpiresAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("ux_artifact_sets_repo_namespace_type_set_id")
                    .table(ArtifactSets::Table)
                    .col(ArtifactSets::Repo)
                    .col(ArtifactSets::Namespace)
                    .col(ArtifactSets::ObjectType)
                    .col(ArtifactSets::ArtifactSetId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_artifact_sets_repo_namespace_type_created")
                    .table(ArtifactSets::Table)
                    .col(ArtifactSets::Repo)
                    .col(ArtifactSets::Namespace)
                    .col(ArtifactSets::ObjectType)
                    .col(ArtifactSets::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ArtifactSetFiles::Table)
                    .if_not_exists()
                    .col(big_integer(ArtifactSetFiles::SetId))
                    .col(text(ArtifactSetFiles::Path))
                    .col(string(ArtifactSetFiles::Oid))
                    .col(big_integer(ArtifactSetFiles::SizeBytes))
                    .col(text_null(ArtifactSetFiles::ContentType))
                    .primary_key(
                        Index::create()
                            .name("pk_artifact_set_files")
                            .table(ArtifactSetFiles::Table)
                            .col(ArtifactSetFiles::SetId)
                            .col(ArtifactSetFiles::Path),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_artifact_set_files_set_id")
                            .from(ArtifactSetFiles::Table, ArtifactSetFiles::SetId)
                            .to(ArtifactSets::Table, ArtifactSets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_artifact_set_files_oid")
                            .from(ArtifactSetFiles::Table, ArtifactSetFiles::Oid)
                            .to(ArtifactObjects::Table, ArtifactObjects::Oid)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_artifact_set_files_oid")
                    .table(ArtifactSetFiles::Table)
                    .col(ArtifactSetFiles::Oid)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ArtifactSetFiles::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ArtifactSets::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ArtifactObjects::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ArtifactObjects {
    Table,
    Oid,
    SizeBytes,
    ContentType,
    StorageKey,
    CreatedAt,
    LastSeenAt,
    Integrity,
}

#[derive(DeriveIden)]
enum ArtifactSets {
    Table,
    Id,
    Repo,
    Namespace,
    ObjectType,
    ArtifactSetId,
    Metadata,
    CreatedBy,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum ArtifactSetFiles {
    Table,
    SetId,
    Path,
    Oid,
    SizeBytes,
    ContentType,
}
