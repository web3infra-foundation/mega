use clap::Args;

use common::enums::DataSource;
use jupiter::context::Context;

#[derive(Args, Clone, Debug)]
pub struct InitOptions {
    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
}

pub async fn init_monorepo(options: &InitOptions) -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new(&options.data_source).await;
    context.services.mega_storage.init_monorepo().await;
    Ok(())
}
