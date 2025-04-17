pub mod consumer;

use std::env;

pub use consumer::new_consumer;
use rdkafka::consumer::StreamConsumer;

pub fn get_consumer() -> StreamConsumer {
    let brokers = env::var("KAFKA_BROKER").unwrap();
    let group_id = env::var("KAFKA_GROUP_ID").unwrap();
    consumer::new_consumer(&brokers, &group_id)
}
