use sea_orm_migration::{prelude::*, schema::*};


#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add columns to MegaBlob table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .add_column(
                        ColumnDef::new(MegaBlob::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .add_column(
                        ColumnDef::new(MegaBlob::FilePath)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .add_column(
                        ColumnDef::new(MegaBlob::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .add_column(
                        ColumnDef::new(MegaBlob::IsDeltaInPack)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to GitBlob table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .add_column(
                        ColumnDef::new(GitBlob::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .add_column(
                        ColumnDef::new(GitBlob::FilePath)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .add_column(
                        ColumnDef::new(GitBlob::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .add_column(
                        ColumnDef::new(GitBlob::IsDeltaInPack)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to MegaCommit table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaCommit::Table)
                    .add_column(
                        ColumnDef::new(MegaCommit::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaCommit::Table)
                    .add_column(
                        ColumnDef::new(MegaCommit::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to GitCommit table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitCommit::Table)
                    .add_column(
                        ColumnDef::new(GitCommit::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitCommit::Table)
                    .add_column(
                        ColumnDef::new(GitCommit::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to MegaTag table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTag::Table)
                    .add_column(
                        ColumnDef::new(MegaTag::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaTag::Table)
                    .add_column(
                        ColumnDef::new(MegaTag::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to GitTag table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitTag::Table)
                    .add_column(
                        ColumnDef::new(GitTag::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitTag::Table)
                    .add_column(
                        ColumnDef::new(GitTag::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to MegaTree table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .add_column(
                        ColumnDef::new(MegaTree::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .add_column(
                        ColumnDef::new(MegaTree::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add columns to GitTree table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitTree::Table)
                    .add_column(
                        ColumnDef::new(GitTree::PackId)
                            .string()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GitTree::Table)
                    .add_column(
                        ColumnDef::new(GitTree::PackOffset)
                            .big_integer()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
       
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns from MegaBlob table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .drop_column(MegaBlob::IsDeltaInPack)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .drop_column(MegaBlob::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .drop_column(MegaBlob::FilePath)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaBlob::Table)
                    .drop_column(MegaBlob::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from MegaCommit table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaCommit::Table)
                    .drop_column(MegaCommit::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaCommit::Table)
                    .drop_column(MegaCommit::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from MegaTag table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTag::Table)
                    .drop_column(MegaTag::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTag::Table)
                    .drop_column(MegaTag::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from MegaTree table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .drop_column(MegaTree::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MegaTree::Table)
                    .drop_column(MegaTree::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from GitBlob table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .drop_column(GitBlob::IsDeltaInPack)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .drop_column(GitBlob::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .drop_column(GitBlob::FilePath)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitBlob::Table)
                    .drop_column(GitBlob::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from GitCommit table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitCommit::Table)
                    .drop_column(GitCommit::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitCommit::Table)
                    .drop_column(GitCommit::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from GitTag table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitTag::Table)
                    .drop_column(GitTag::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitTag::Table)
                    .drop_column(GitTag::PackId)
                    .to_owned(),
            )
            .await?;

        // Drop columns from GitTree table one by one for SQLite compatibility
        manager
            .alter_table(
                Table::alter()
                    .table(GitTree::Table)
                    .drop_column(GitTree::PackOffset)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GitTree::Table)
                    .drop_column(GitTree::PackId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum MegaBlob {
    Table,
    PackId,
    FilePath,
    PackOffset,
    IsDeltaInPack,
}
#[derive(Iden)]
enum GitBlob {
    Table,
    PackId,
    FilePath,
    PackOffset,
    IsDeltaInPack,
}

#[derive(Iden)]
enum MegaCommit {
    Table,
    PackId,
    PackOffset,
}

#[derive(Iden)]
enum GitCommit {
    Table,
    PackId,
    PackOffset,
}

#[derive(Iden)]
enum MegaTag {
    Table,
    PackId,
    PackOffset,
}
#[derive(Iden)]
enum GitTag {
    Table,
    PackId,
    PackOffset,
}


#[derive(Iden)]
enum MegaTree {
    Table,
    PackId,
    PackOffset,
}

#[derive(Iden)]
enum GitTree {
    Table,
    PackId,
    PackOffset,
}