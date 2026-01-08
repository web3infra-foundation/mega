use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                r#"DROP TYPE IF EXISTS storage_type_enum;"#.to_owned(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                r#"
                    CREATE TYPE storage_type_enum AS ENUM (
                        'database',
                        'local_fs',
                        'aws_s3'
                    );
                    "#
                .to_owned(),
            ))
            .await?;

        Ok(())
    }
}
