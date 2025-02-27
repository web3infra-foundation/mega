use crate::queue::{MessageQueue, MQ};
use common::config::Config;
use jupiter::context::Context;

pub async fn init_mq(config: &Config) {
    let ctx = Context::new(config.clone()).await;
    let seq = match ctx.services.mq_storage.get_latest_message().await {
        Some(model) => model.id + 1,
        None => 1,
    };

    let mq = MessageQueue::new(seq, ctx);
    mq.start();

    MQ.set(mq).unwrap();
}
