use chrono::format::Colons::Colon;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk-gpg_key-user_id")
                    .table(Gpgkey::Table)
                    .to_owned()
            ).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gpgkey::Table)
                    .modify_column(ColumnDef::new(Gpgkey::UserId).string().not_null())
                    .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Gpgkey::Table)
                    .modify_column(ColumnDef::new(Gpgkey::UserId).big_integer().not_null())
                    .to_owned()
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_gpg_user_id") // original FK name
                    .from(Gpgkey::Table, Gpgkey::UserId)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned()
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Gpgkey {
    #[sea_orm(iden = "gpg_key")]
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
