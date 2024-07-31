use common::config::Config;

use jupiter::context::Context;

pub async fn init_monorepo(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new(config).await;
    context.services.mega_storage.init_monorepo().await;
    Ok(())
}

pub fn init_message_queue() -> Result<(), Box<dyn std::error::Error>> {
    crate::mq::queue::init_message_queue(12);
    Ok(())
}
