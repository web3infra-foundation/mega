use sea_orm::{DatabaseBackend, EnumIter, Iterable, sea_query::extension::postgres::Type};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if let DatabaseBackend::Postgres = manager.get_database_backend() {
            let conn = manager.get_connection();

            manager
                .create_type(
                    Type::create()
                        .as_enum(WebhookEventTypeEnum)
                        .values(WebhookEventTypeVariant::iter())
                        .to_owned(),
                )
                .await?;

            conn.execute_unprepared(
                r#"ALTER TABLE mega_webhook_delivery
                    ALTER COLUMN event_type TYPE webhook_event_type_enum
                    USING event_type::webhook_event_type_enum"#,
            )
            .await?;

            conn.execute_unprepared(
                r#"ALTER TABLE mega_webhook_event_type
                    ALTER COLUMN event_type TYPE webhook_event_type_enum
                    USING event_type::webhook_event_type_enum"#,
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if let DatabaseBackend::Postgres = manager.get_database_backend() {
            let conn = manager.get_connection();

            conn.execute_unprepared(
                r#"ALTER TABLE mega_webhook_delivery
                    ALTER COLUMN event_type TYPE varchar USING event_type::text"#,
            )
            .await?;

            conn.execute_unprepared(
                r#"ALTER TABLE mega_webhook_event_type
                    ALTER COLUMN event_type TYPE varchar USING event_type::text"#,
            )
            .await?;

            manager
                .drop_type(
                    Type::drop()
                        .if_exists()
                        .name(WebhookEventTypeEnum)
                        .restrict()
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
struct WebhookEventTypeEnum;

#[derive(Iden, EnumIter)]
enum WebhookEventTypeVariant {
    #[iden = "cl.created"]
    ClCreated,
    #[iden = "cl.updated"]
    ClUpdated,
    #[iden = "cl.merged"]
    ClMerged,
    #[iden = "cl.closed"]
    ClClosed,
    #[iden = "cl.reopened"]
    ClReopened,
    #[iden = "cl.comment.created"]
    ClCommentCreated,
    #[iden = "all"]
    All,
}
