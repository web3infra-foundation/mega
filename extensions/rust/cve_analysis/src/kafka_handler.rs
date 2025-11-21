use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::{KafkaError, KafkaResult};
use rdkafka::message::{BorrowedMessage, Headers};
use rdkafka::producer::{BaseProducer, BaseRecord, ProducerContext};
use rdkafka::util::Timeout;
use rdkafka::{ClientContext, Message, TopicPartitionList};
use std::time::Duration;

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
        // match result {
        //     Ok(delivery) => tracing::info!("Delivered message to {:?}", delivery),
        //     Err((error, _)) => tracing::error!("Failed to deliver message: {:?}", error),
        // }
    }
}
#[allow(dead_code)]
pub enum KafkaHandler {
    Consumer(BaseConsumer<CustomContext>),
    Producer(BaseProducer<CustomContext>),
}
impl KafkaHandler {
    pub fn new_consumer(brokers: &str, group_id: &str, topic: &str) -> Result<Self, KafkaError> {
        let context = CustomContext;

        let consumer: BaseConsumer<CustomContext> = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "10000")
            .set("heartbeat.interval.ms", "1500")
            .set("max.poll.interval.ms", "3000000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set_log_level(RDKafkaLogLevel::Debug)
            .create_with_context(context)?;

        consumer.subscribe(&[topic])?;

        Ok(KafkaHandler::Consumer(consumer))
    }

    pub async fn consume_once(&'_ self) -> Result<BorrowedMessage<'_>, KafkaError> {
        if let KafkaHandler::Consumer(consumer) = self {
            tracing::debug!("Trying to consume a message");

            match consumer.poll(Duration::from_secs(0)) {
                None => {
                    //tracing::info!("No message received");
                    Err(KafkaError::NoMessageReceived)
                }
                Some(m) => {
                    let m = m?;
                    tracing::debug!("{:?}", m);
                    if let Some(headers) = m.headers() {
                        for header in headers.iter() {
                            tracing::info!("Header {}: {:?}", header.key, header.value);
                        }
                    }
                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                    Ok(m)
                }
            }
        } else {
            unreachable!("Called consume_once on a producer");
        }
    }
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
