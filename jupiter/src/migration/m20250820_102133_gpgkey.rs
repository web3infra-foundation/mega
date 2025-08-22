use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        println!("uploading gpg migration");
        manager
            .create_table(
                Table::create()
                    .table(GpgKey::Table)
                    .if_not_exists()
                    .col(pk_auto(GpgKey::Id))
                    .col(big_integer(GpgKey::UserId))
                    .col(text(GpgKey::KeyId))
                    .col(text(GpgKey::PublicKey))
                    .col(text(GpgKey::Fingerprint).unique_key())
                    .col(text(GpgKey::Alias))
                    .col(timestamp(GpgKey::CreatedAt))
                    .col(timestamp_null(GpgKey::ExpiresAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-gpg_key-user_id")
                            .from(GpgKey::Table, GpgKey::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GpgKey::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum GpgKey {
    Table,
    Id,
    KeyId,
    UserId,
    PublicKey,
    Fingerprint,
    Alias,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
