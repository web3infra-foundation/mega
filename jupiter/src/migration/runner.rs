use common::errors::MegaError;
use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::MigratorTrait;
use tracing::log;

use super::Migrator;

/// Applies database migrations to the given database connection.
pub async fn apply_migrations(db: &DatabaseConnection, refresh: bool) -> Result<(), MegaError> {
    match refresh {
        true => Migrator::refresh(db).await,
        false => Migrator::up(db, None).await,
    }
    .map_err(|e| {
        log::error!("Failed to apply migrations: {e}");
        e.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_db_connection;

    #[tokio::test]
    async fn test_apply_migrations() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temporary directory");
        let db = test_db_connection(temp_dir.path()).await;
        let result = apply_migrations(&db, false).await;
        assert!(
            result.is_ok(),
            "Failed to apply migrations: {:?}",
            result.err()
        );

        let applied_migrations = Migrator::get_applied_migrations(&db).await.unwrap();
        assert!(!applied_migrations.is_empty(), "No migrations were applied");
    }
}
