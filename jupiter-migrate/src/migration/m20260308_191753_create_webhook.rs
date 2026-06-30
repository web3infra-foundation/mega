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
                    .table(MegaWebhook::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaWebhook::Id))
                    .col(string(MegaWebhook::TargetUrl))
                    .col(string(MegaWebhook::Secret))
                    .col(text(MegaWebhook::EventTypes))
                    .col(string_null(MegaWebhook::PathFilter))
                    .col(boolean(MegaWebhook::Active))
                    .col(date_time(MegaWebhook::CreatedAt))
                    .col(date_time(MegaWebhook::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaWebhookDelivery::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaWebhookDelivery::Id))
                    .col(big_integer(MegaWebhookDelivery::WebhookId))
                    .col(string(MegaWebhookDelivery::EventType))
                    .col(text(MegaWebhookDelivery::Payload))
                    .col(integer_null(MegaWebhookDelivery::ResponseStatus))
                    .col(text_null(MegaWebhookDelivery::ResponseBody))
                    .col(boolean(MegaWebhookDelivery::Success))
                    .col(integer(MegaWebhookDelivery::Attempt))
                    .col(text_null(MegaWebhookDelivery::ErrorMessage))
                    .col(date_time(MegaWebhookDelivery::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_delivery_webhook_id")
                            .from(MegaWebhookDelivery::Table, MegaWebhookDelivery::WebhookId)
                            .to(MegaWebhook::Table, MegaWebhook::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_webhook_delivery_webhook_id")
                    .table(MegaWebhookDelivery::Table)
                    .col(MegaWebhookDelivery::WebhookId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MegaWebhookDelivery::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MegaWebhook::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaWebhook {
    Table,
    Id,
    TargetUrl,
    Secret,
    EventTypes,
    PathFilter,
    Active,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MegaWebhookDelivery {
    Table,
    Id,
    WebhookId,
    EventType,
    Payload,
    ResponseStatus,
    ResponseBody,
    Success,
    Attempt,
    ErrorMessage,
    CreatedAt,
}
