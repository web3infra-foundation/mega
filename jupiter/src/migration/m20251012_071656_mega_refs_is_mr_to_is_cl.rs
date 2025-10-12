use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // rename column is_mr -> is_cl in mega_refs
        manager
            .alter_table(
                Table::alter()
                    .table(MegaRefs::Table)
                    .rename_column(MegaRefsColumn::IsMr, MegaRefsColumn::IsCl)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // reverse rename: is_cl -> is_mr
        manager
            .alter_table(
                Table::alter()
                    .table(MegaRefs::Table)
                    .rename_column(MegaRefsColumn::IsCl, MegaRefsColumn::IsMr)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaRefs {
    Table,
}

#[derive(DeriveIden)]
enum MegaRefsColumn {
    IsMr,
    IsCl,
}
