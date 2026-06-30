use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_bot_tokens_token_hash")
                    .table(BotTokens::Table)
                    .col(BotTokens::TokenHash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_bot_tokens_token_hash")
                    .table(BotTokens::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum BotTokens {
    Table,
    TokenHash,
}
