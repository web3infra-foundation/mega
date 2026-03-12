use std::collections::HashSet;

use sea_orm::{FromQueryResult, Statement};
use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, string},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Debug, FromQueryResult)]
struct WebhookRow {
    id: i64,
    event_types: String,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MegaWebhookEventType::Table)
                    .if_not_exists()
                    .col(big_integer(MegaWebhookEventType::WebhookId))
                    .col(string(MegaWebhookEventType::EventType))
                    .primary_key(
                        Index::create()
                            .col(MegaWebhookEventType::WebhookId)
                            .col(MegaWebhookEventType::EventType),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_event_type_webhook_id")
                            .from(MegaWebhookEventType::Table, MegaWebhookEventType::WebhookId)
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
                    .name("idx_webhook_event_type_event_type")
                    .table(MegaWebhookEventType::Table)
                    .col(MegaWebhookEventType::EventType)
                    .to_owned(),
            )
            .await?;

        let conn = manager.get_connection();
        let webhooks: Vec<WebhookRow> = WebhookRow::find_by_statement(Statement::from_string(
            manager.get_database_backend(),
            "SELECT id, event_types FROM mega_webhook".to_string(),
        ))
        .all(conn)
        .await?;

        for webhook in webhooks {
            let parsed: Vec<String> =
                serde_json::from_str(&webhook.event_types).unwrap_or_default();
            let mut dedup = HashSet::new();
            for event_type in parsed {
                if event_type.is_empty() || !dedup.insert(event_type.clone()) {
                    continue;
                }
                let insert = Query::insert()
                    .into_table(MegaWebhookEventType::Table)
                    .columns([
                        MegaWebhookEventType::WebhookId,
                        MegaWebhookEventType::EventType,
                    ])
                    .values_panic([webhook.id.into(), event_type.into()])
                    .to_owned();
                manager.exec_stmt(insert).await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MegaWebhookEventType::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MegaWebhook {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum MegaWebhookEventType {
    Table,
    WebhookId,
    EventType,
}
