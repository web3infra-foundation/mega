use sea_orm_migration::{prelude::*, schema::*};

use crate::migration::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(Reactions::Table)
                    .col(pk_bigint(Reactions::Id))
                    .col(string(Reactions::PublicId))
                    .col(string_null(Reactions::Content))
                    .col(big_integer(Reactions::SubjectId))
                    .col(string(Reactions::SubjectType))
                    .col(big_integer_null(Reactions::OrganizationMembershipId))
                    .col(string(Reactions::Username))
                    .col(date_time_null(Reactions::DiscardedAt))
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
enum Reactions {
    Table,
    Id,
    PublicId,
    Content,
    SubjectId,
    SubjectType,
    OrganizationMembershipId,
    Username,
    DiscardedAt,
}
