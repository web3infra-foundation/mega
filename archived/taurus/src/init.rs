use crate::queue::{MessageQueue, MQ};
use jupiter::context::Context;

pub async fn init_mq(ctx: Context) {
    let seq = match ctx.services.mq_storage.get_latest_message().await {
        Some(model) => model.id + 1,
        None => 1,
    };

    let mq = MessageQueue::new(seq, ctx);
    mq.start();

    MQ.set(mq).unwrap();
}
