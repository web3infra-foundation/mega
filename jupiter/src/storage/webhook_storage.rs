use std::ops::Deref;

use api_model::common::Pagination;
use callisto::{
    mega_webhook, mega_webhook_delivery, mega_webhook_event_type,
    sea_orm_active_enums::WebhookEventTypeEnum,
};
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct WebhookWithEventTypes {
    pub webhook: mega_webhook::Model,
    pub event_types: Vec<WebhookEventTypeEnum>,
}

#[derive(Clone)]
pub struct WebhookStorage {
    pub base: BaseStorage,
}

impl Deref for WebhookStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl WebhookStorage {
    pub async fn create_webhook(
        &self,
        mut model: mega_webhook::Model,
        event_types: Vec<WebhookEventTypeEnum>,
    ) -> Result<WebhookWithEventTypes, MegaError> {
        let normalized_event_types = normalize_event_types(event_types);
        model.event_types = serde_json::to_string(&normalized_event_types)?;
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        self.replace_event_types(res.id, &normalized_event_types)
            .await?;

        Ok(WebhookWithEventTypes {
            webhook: res,
            event_types: normalized_event_types,
        })
    }

    pub async fn list_webhooks(
        &self,
        page: Pagination,
    ) -> Result<(Vec<WebhookWithEventTypes>, u64), MegaError> {
        let paginator = mega_webhook::Entity::find()
            .order_by_desc(mega_webhook::Column::CreatedAt)
            .paginate(self.get_connection(), page.per_page);
        let total = paginator.num_items().await?;
        let webhooks = paginator.fetch_page(page.page.saturating_sub(1)).await?;
        let event_types_map = self.event_types_map(&webhooks).await?;
        let items = webhooks
            .into_iter()
            .map(|webhook| {
                let event_types = event_types_map
                    .get(&webhook.id)
                    .cloned()
                    .unwrap_or_default();
                WebhookWithEventTypes {
                    webhook,
                    event_types,
                }
            })
            .collect();
        Ok((items, total))
    }

    pub async fn get_webhook(&self, id: i64) -> Result<Option<mega_webhook::Model>, MegaError> {
        let model = mega_webhook::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    pub async fn delete_webhook(&self, id: i64) -> Result<(), MegaError> {
        let delete_result = mega_webhook::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await?;

        if delete_result.rows_affected == 0 {
            return Err(MegaError::NotFound(format!(
                "Webhook with id `{id}` not found"
            )));
        }

        Ok(())
    }

    pub async fn find_matching_webhooks(
        &self,
        event_type: WebhookEventTypeEnum,
        path: &str,
    ) -> Result<Vec<mega_webhook::Model>, MegaError> {
        let candidates = mega_webhook::Entity::find()
            .join(
                JoinType::InnerJoin,
                mega_webhook::Relation::WebhookEventTypes.def(),
            )
            .filter(mega_webhook::Column::Active.eq(true))
            .filter(
                mega_webhook_event_type::Column::EventType
                    .is_in([event_type, WebhookEventTypeEnum::All]),
            )
            .distinct()
            .all(self.get_connection())
            .await?;

        let matching = candidates
            .into_iter()
            .filter(|w| {
                if let Some(ref filter) = w.path_filter {
                    path.starts_with(filter.as_str())
                } else {
                    true
                }
            })
            .collect();

        Ok(matching)
    }

    pub async fn save_delivery(
        &self,
        model: mega_webhook_delivery::Model,
    ) -> Result<(), MegaError> {
        model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(())
    }

    async fn replace_event_types(
        &self,
        webhook_id: i64,
        event_types: &[WebhookEventTypeEnum],
    ) -> Result<(), MegaError> {
        mega_webhook_event_type::Entity::delete_many()
            .filter(mega_webhook_event_type::Column::WebhookId.eq(webhook_id))
            .exec(self.get_connection())
            .await?;

        for event_type in event_types {
            mega_webhook_event_type::ActiveModel {
                webhook_id: Set(webhook_id),
                event_type: Set(*event_type),
            }
            .insert(self.get_connection())
            .await?;
        }
        Ok(())
    }

    async fn event_types_map(
        &self,
        webhooks: &[mega_webhook::Model],
    ) -> Result<std::collections::HashMap<i64, Vec<WebhookEventTypeEnum>>, MegaError> {
        let mut map = std::collections::HashMap::new();
        if webhooks.is_empty() {
            return Ok(map);
        }

        let ids: Vec<i64> = webhooks.iter().map(|w| w.id).collect();
        let rows = mega_webhook_event_type::Entity::find()
            .filter(mega_webhook_event_type::Column::WebhookId.is_in(ids))
            .all(self.get_connection())
            .await?;

        for row in rows {
            map.entry(row.webhook_id)
                .or_insert_with(Vec::new)
                .push(row.event_type);
        }

        Ok(map)
    }
}

fn normalize_event_types(event_types: Vec<WebhookEventTypeEnum>) -> Vec<WebhookEventTypeEnum> {
    let mut dedup = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for event in event_types {
        if dedup.insert(event) {
            normalized.push(event);
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use api_model::common::Pagination;
    use chrono::Utc;
    use idgenerator::IdInstance;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        migration::apply_migrations,
        tests::{test_db_connection, test_storage},
    };

    #[test]
    fn test_normalize_event_types_dedupes() {
        let normalized = normalize_event_types(vec![
            WebhookEventTypeEnum::ClCreated,
            WebhookEventTypeEnum::ClCreated,
            WebhookEventTypeEnum::All,
            WebhookEventTypeEnum::All,
        ]);
        assert_eq!(
            normalized,
            vec![WebhookEventTypeEnum::ClCreated, WebhookEventTypeEnum::All]
        );
    }

    #[tokio::test]
    async fn test_find_matching_webhooks_uses_normalized_event_types() {
        let temp_dir = TempDir::new().expect("failed to create temporary directory");
        let conn = test_db_connection(temp_dir.path()).await;
        apply_migrations(&conn, true).await.unwrap();
        let storage = WebhookStorage {
            base: BaseStorage::new(std::sync::Arc::new(conn)),
        };

        let now = Utc::now().naive_utc();
        let created = storage
            .create_webhook(
                mega_webhook::Model {
                    id: IdInstance::next_id(),
                    target_url: "https://example.com/a".to_string(),
                    secret: "s1".to_string(),
                    event_types: "[]".to_string(),
                    path_filter: Some("/repo".to_string()),
                    active: true,
                    created_at: now,
                    updated_at: now,
                },
                vec![
                    WebhookEventTypeEnum::ClCreated,
                    WebhookEventTypeEnum::All,
                    WebhookEventTypeEnum::ClCreated,
                ],
            )
            .await
            .unwrap();

        storage
            .create_webhook(
                mega_webhook::Model {
                    id: IdInstance::next_id(),
                    target_url: "https://example.com/b".to_string(),
                    secret: "s2".to_string(),
                    event_types: "[]".to_string(),
                    path_filter: Some("/repo".to_string()),
                    active: true,
                    created_at: now,
                    updated_at: now,
                },
                vec![WebhookEventTypeEnum::ClUpdated],
            )
            .await
            .unwrap();

        let (items, total) = storage
            .list_webhooks(Pagination {
                page: 1,
                per_page: 20,
            })
            .await
            .unwrap();
        assert_eq!(total, 2);
        let created_item = items
            .into_iter()
            .find(|item| item.webhook.id == created.webhook.id)
            .unwrap();
        let mut event_types = created_item.event_types;
        event_types.sort_by_key(|e| format!("{e:?}"));
        assert_eq!(
            event_types,
            vec![WebhookEventTypeEnum::All, WebhookEventTypeEnum::ClCreated]
        );

        let created_matches = storage
            .find_matching_webhooks(WebhookEventTypeEnum::ClCreated, "/repo/app")
            .await
            .unwrap();
        assert_eq!(created_matches.len(), 1);
        assert_eq!(created_matches[0].id, created.webhook.id);

        let updated_matches = storage
            .find_matching_webhooks(WebhookEventTypeEnum::ClUpdated, "/repo/app")
            .await
            .unwrap();
        assert_eq!(updated_matches.len(), 2);

        let outside_path_matches = storage
            .find_matching_webhooks(WebhookEventTypeEnum::ClUpdated, "/outside/path")
            .await
            .unwrap();
        assert!(outside_path_matches.is_empty());
    }

    #[tokio::test]
    async fn test_create_list_delete_webhook_lifecycle() {
        let temp_dir = TempDir::new().expect("failed to create temporary directory");
        let storage = test_storage(temp_dir.path()).await;
        let webhook_storage = storage.webhook_storage();

        let now = Utc::now().naive_utc();
        let webhook_a = webhook_storage
            .create_webhook(
                mega_webhook::Model {
                    id: IdInstance::next_id(),
                    target_url: "https://example.com/a".to_string(),
                    secret: "s1".to_string(),
                    event_types: "[]".to_string(),
                    path_filter: Some("/repo".to_string()),
                    active: true,
                    created_at: now,
                    updated_at: now,
                },
                vec![WebhookEventTypeEnum::ClCreated],
            )
            .await
            .unwrap();
        let webhook_b = webhook_storage
            .create_webhook(
                mega_webhook::Model {
                    id: IdInstance::next_id(),
                    target_url: "https://example.com/b".to_string(),
                    secret: "s2".to_string(),
                    event_types: "[]".to_string(),
                    path_filter: Some("/repo".to_string()),
                    active: true,
                    created_at: now,
                    updated_at: now,
                },
                vec![WebhookEventTypeEnum::ClUpdated],
            )
            .await
            .unwrap();

        let (before_delete, total_before_delete) = webhook_storage
            .list_webhooks(Pagination {
                page: 1,
                per_page: 20,
            })
            .await
            .unwrap();
        assert_eq!(total_before_delete, 2);
        assert_eq!(before_delete.len(), 2);

        webhook_storage
            .delete_webhook(webhook_a.webhook.id)
            .await
            .unwrap();
        assert!(
            webhook_storage
                .get_webhook(webhook_a.webhook.id)
                .await
                .unwrap()
                .is_none()
        );

        let (after_delete, total_after_delete) = webhook_storage
            .list_webhooks(Pagination {
                page: 1,
                per_page: 20,
            })
            .await
            .unwrap();
        assert_eq!(total_after_delete, 1);
        assert_eq!(after_delete.len(), 1);
        assert_eq!(after_delete[0].webhook.id, webhook_b.webhook.id);

        let err = webhook_storage
            .delete_webhook(webhook_a.webhook.id)
            .await
            .unwrap_err();
        assert!(matches!(err, MegaError::NotFound(_)));
    }
}
