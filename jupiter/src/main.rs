use jupiter::migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    cli::run_cli(migrator::Migrator).await;
}
