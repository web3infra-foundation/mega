pub use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::big_integer;

mod m20250314_025943_init;
mod m20250427_031332_add_mr_refs_tag;
mod m20250605_013340_alter_mega_mr_index;
mod m20250613_033821_alter_user_id;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250314_025943_init::Migration),
            Box::new(m20250427_031332_add_mr_refs_tag::Migration),
            Box::new(m20250605_013340_alter_mega_mr_index::Migration),
            Box::new(m20250613_033821_alter_user_id::Migration),
        ]
    }
}

pub fn pk_bigint<T: IntoIden>(name: T) -> ColumnDef {
    big_integer(name).primary_key().take()
}
