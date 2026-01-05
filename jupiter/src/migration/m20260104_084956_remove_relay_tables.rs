use sea_orm_migration::prelude::*;

/// Migration struct for the operation
/// This migration is intended to drop obsolete tables from the database
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// It will drop the specified tables from the database.
    /// The tables will be dropped with idempotency, ensuring no error if the table doesn't exist.
    ///
    /// Purpose: Drop outdated or unused tables to clean up the database.
    /// Reason: The tables are no longer in use and should be removed to maintain a cleaner and more efficient database.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop table relay_lfs_info if it exists
        manager
            .drop_table(Table::drop().table(RelayLfsInfo::Table).to_owned())
            .await?;

        // Drop table relay_node if it exists
        manager
            .drop_table(Table::drop().table(RelayNode::Table).to_owned())
            .await?;

        // Drop table relay_nostr_event if it exists
        manager
            .drop_table(Table::drop().table(RelayNostrEvent::Table).to_owned())
            .await?;

        // Drop table relay_nostr_req if it exists
        manager
            .drop_table(Table::drop().table(RelayNostrReq::Table).to_owned())
            .await?;

        // Drop table relay_path_mapping if it exists
        manager
            .drop_table(Table::drop().table(RelayPathMapping::Table).to_owned())
            .await?;

        // Drop table relay_repo_info if it exists
        manager
            .drop_table(Table::drop().table(RelayRepoInfo::Table).to_owned())
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
