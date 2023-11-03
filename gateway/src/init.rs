use clap::Args;
use storage::driver::database::{self, DataSource};
#[derive(Args, Clone, Debug)]
pub struct InitOptions {
    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
}

pub async fn init_dir(options: &InitOptions) -> Result<(), Box<dyn std::error::Error>> {
    let storage = database::init(&options.data_source).await;
    storage.init_repo_dir().await.unwrap();
    Ok(())
}
