use common::config::Config;
use jupiter::context::Context;
use crate::queue::{MessageQueue, MQ};

pub async fn init_mq(config: &Config) {
    let ctx = Context::new(config.clone()).await;
    let mq = MessageQueue::new(12, ctx);
    mq.start();

    MQ.set(mq).unwrap();
}
