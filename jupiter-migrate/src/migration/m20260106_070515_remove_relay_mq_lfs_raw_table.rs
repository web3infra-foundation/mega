use sea_orm_migration::prelude::*;

/// Cleanup legacy database tables related to deprecated Relay, MQ, and LFS split features.
///
/// Background:
/// Over multiple iterations of the Mega system architecture, several experimental
/// or deprecated data models have become unused. These tables were originally created
/// to support early Relay / Nostr experiments, temporary message storage, or legacy LFS
/// split strategies, and are now fully replaced by newer mechanisms.
///
/// Purpose:
/// This migration removes obsolete tables in order to simplify the schema,
/// reduce maintenance overhead, and avoid confusion around models that are no longer
/// part of the active architecture.
///
/// The following categories of tables are removed:
///
/// 1) Relay / Nostr related tables (no longer used)
///    - relay_lfs_info
///    - relay_node
///    - relay_nostr_event
///    - relay_nostr_req
///    - relay_path_mapping
///    - relay_repo_info
///
/// 2) Deprecated MQ storage table
///    - mq_storage
///
/// 3) Legacy LFS split relationship table
///    - lfs_split_relations
///
/// 4) Old Git object storage table
///    - raw_blob
///
/// All drop operations are idempotent — if a table does not exist, the migration
/// will still execute successfully without errors.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop relay-related tables
        manager
            .drop_table(Table::drop().table(RelayLfsInfo::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RelayNode::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RelayNostrEvent::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RelayNostrReq::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RelayPathMapping::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RelayRepoInfo::Table).to_owned())
            .await?;

        // Drop MqStorage table
        manager
            .drop_table(Table::drop().table(MqStorage::Table).to_owned())
            .await?;

        // Drop lfs_split_relation table
        manager
            .drop_table(Table::drop().table(LfsSplitRelations::Table).to_owned())
            .await?;

        // Drop raw_blow table
        manager
            .drop_table(Table::drop().table(RawBlob::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum RelayLfsInfo {
    #[sea_orm(iden = "relay_lfs_info")]
    Table,
}

#[derive(DeriveIden)]
enum RelayNode {
    #[sea_orm(iden = "relay_node")]
    Table,
}

#[derive(DeriveIden)]
enum RelayNostrEvent {
    #[sea_orm(iden = "relay_nostr_event")]
    Table,
}

#[derive(DeriveIden)]
enum RelayNostrReq {
    #[sea_orm(iden = "relay_nostr_req")]
    Table,
}

#[derive(DeriveIden)]
enum RelayPathMapping {
    #[sea_orm(iden = "relay_path_mapping")]
    Table,
}

#[derive(DeriveIden)]
enum RelayRepoInfo {
    #[sea_orm(iden = "relay_repo_info")]
    Table,
}

#[derive(DeriveIden)]
enum MqStorage {
    #[sea_orm(iden = "mq_storage")]
    Table,
}

#[derive(DeriveIden)]
enum LfsSplitRelations {
    #[sea_orm(iden = "lfs_split_relations")]
    Table,
}

#[derive(DeriveIden)]
enum RawBlob {
    #[sea_orm(iden = "raw_blob")]
    Table,
}
