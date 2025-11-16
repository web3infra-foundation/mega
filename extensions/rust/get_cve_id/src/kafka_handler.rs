use std::time::Duration;

use rdkafka::config::{ClientConfig};
use rdkafka::consumer::{BaseConsumer, ConsumerContext, Rebalance};
use rdkafka::error::{KafkaError, KafkaResult};

use rdkafka::producer::{BaseProducer, BaseRecord, ProducerContext};
use rdkafka::util::Timeout;
use rdkafka::{ClientContext,  TopicPartitionList};

#[derive(Clone)]
pub struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, _: &BaseConsumer<CustomContext>, rebalance: &Rebalance) {
        tracing::info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, _: &BaseConsumer<CustomContext>, rebalance: &Rebalance) {
        tracing::info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        tracing::info!("Committing offsets: {:?}", result);
    }
}

impl ProducerContext for CustomContext {
    type DeliveryOpaque = ();

    fn delivery(
        &self,
        _result: &rdkafka::producer::DeliveryResult,
        _delivery_opaque: Self::DeliveryOpaque,
    ) {
        tracing::info!("Delivery result: {:?}", _result);
    }
}
#[allow(dead_code)]
pub enum KafkaHandler {
    Consumer(BaseConsumer<CustomContext>),
    Producer(BaseProducer<CustomContext>),
}
impl KafkaHandler {
    pub fn new_producer(brokers: &str) -> Result<Self, KafkaError> {
        let context = CustomContext;

        let producer: BaseProducer<CustomContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create_with_context(context)?;

        Ok(KafkaHandler::Producer(producer))
    }
    #[allow(irrefutable_let_patterns)]
    pub async fn send_message(&self, topic: &str, key: &str, payload: &str) {
        if let KafkaHandler::Producer(producer) = self {
            let record = BaseRecord::to(topic).key(key).payload(payload);

            match producer.send(record) {
                Ok(_) => {
                    tracing::info!("Message sent successfully");
                }
                Err(e) => tracing::error!("Failed to send message: {:?}", e),
            }

            producer.poll(Timeout::After(Duration::from_secs(1)));
        } else {
            tracing::error!("Called send_message on a consumer");
        }
    }

}