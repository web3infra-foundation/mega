pub use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::big_integer;

mod m20250314_025943_init;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250314_025943_init::Migration)]
    }
}

pub fn pk_bigint<T: IntoIden>(name: T) -> ColumnDef {
    big_integer(name).primary_key().take()
}
