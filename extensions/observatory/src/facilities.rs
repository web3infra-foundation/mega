use std::sync::Arc;

use rdkafka::{
    Message,
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
    error::KafkaError,
    producer::{FutureProducer, FutureRecord},
};
use tokio::time::Duration;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct Satellite {
    producer: Arc<FutureProducer>,
    input_topic: String,
}

impl Satellite {
    pub fn new(broker: &str, input_topic: &str) -> Self {
        Satellite {
            producer: Arc::new(
                ClientConfig::new()
                    .set("bootstrap.servers", broker)
                    .set("message.timeout.ms", "5000")
                    .set("security.protocol", "PLAINTEXT")
                    .create()
                    .expect("Producer creation error"),
            ),
            input_topic: input_topic.to_owned(),
        }
    }

    pub async fn send_message(&self, payload: impl Into<String>) -> Result<(), anyhow::Error> {
        let topic = &self.input_topic;
        let payload = payload.into();
        let record = FutureRecord::to(topic).key("default_key").payload(&payload);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(delivery) => {
                info!(
                    "✅ Kafka message sent to topic '{}': {:?}, payload: {}",
                    topic, delivery, payload
                );
                Ok(())
            }
            Err((err, _)) => {
                error!(
                    "❌ Kafka send error on topic '{}': {:?}, payload: {}",
                    topic, err, payload
                );
                Err(err.into())
            }
        }
    }
}

#[derive(Clone)]
pub struct Telescope {
    consumer: Arc<StreamConsumer>,
}

impl Telescope {
    pub fn new(broker: &str, group_id: &str, output_topic: &str) -> Self {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", broker)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Consumer creation failed");
        consumer
            .subscribe(&[output_topic])
            .expect("Failed to subscribe to topic");

        Self {
            consumer: Arc::new(consumer),
        }
    }

    pub async fn consume_loop<F>(&self, mut handle: F)
    where
        F: AsyncFnMut(String) + Send + 'static,
    {
        let mut stream = self.consumer.stream();

        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(msg) => {
                    if let Some(payload) = msg.payload_view::<str>().transpose().unwrap_or(None) {
                        debug!("✅ Received: {}", payload);
                        handle(payload.to_string()).await;
                        self.consumer
                            .commit_message(&msg, CommitMode::Async)
                            .unwrap();
                    }
                }
                Err(e) => {
                    error!("❌ Kafka error: {}", e);
                }
            }
        }

        tracing::warn!("Kafka stream has ended.");
    }

    pub async fn consume_once<F>(&self, mut handle: F) -> Result<(), KafkaError>
    where
        F: FnMut(String) + Send + 'static,
    {
        let mut stream = self.consumer.stream();

        let timeout = Duration::from_secs(10);
        let maybe_msg = tokio::time::timeout(timeout, stream.next()).await;

        match maybe_msg {
            Ok(Some(Ok(msg))) => {
                if let Some(payload) = msg.payload_view::<str>().transpose().unwrap() {
                    handle(payload.to_string());
                    Ok(())
                } else {
                    Err(KafkaError::NoMessageReceived)
                }
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(KafkaError::NoMessageReceived),
            Err(_) => Err(KafkaError::NoMessageReceived),
        }
    }
}

#[derive(Clone)]
pub struct Station {
    satellite: Satellite,
    telescope: Telescope,
}

impl Station {
    pub fn new(broker: &str, produce_topic: &str, consume_topic: &str, group_id: &str) -> Self {
        let satellite = Satellite::new(broker, produce_topic);
        let telescope = Telescope::new(broker, group_id, consume_topic);
        Self {
            satellite,
            telescope,
        }
    }

    /// Asynchronously processes messages from Kafka, transforms them with a handler,
    /// and sends them using the satellite component.
    ///
    /// # Arguments
    /// * `handle` - A function or closure that transforms the Kafka message payload.
    ///   `  ` It takes a `old payload String` and returns a `new payload String`.
    ///
    /// # Returns
    /// * `Result<(), anyhow::Error>` - Returns `Ok(())` if successful, or a anyhow error.
    pub async fn message_process<F>(&self, mut handle: F) -> Result<(), anyhow::Error>
    where
        F: FnMut(String) -> String + Send + 'static,
    {
        let satellite = self.satellite.clone();
        self.telescope
            .consume_loop(move |payload| {
                let satellite = satellite.clone();
                let new_payload = handle(payload);
                async move {
                    // ignore send msg err
                    let _ = satellite.send_message(&new_payload).await;
                }
            })
            .await;
        Ok(())
    }
}
