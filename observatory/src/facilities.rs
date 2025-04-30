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

#[cfg(test)]
mod test {
    use crate::{
        facilities::{Satellite, Telescope},
        model::{DataSource, crate_repo::CrateRepoMessage, crates::CrateMessage},
    };
    use chrono::Utc;
    use std::sync::Arc;
    use std::sync::Once;
    use testcontainers::{
        ContainerAsync, GenericImage, ImageExt,
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
    };

    use tokio::time::Duration;
    use uuid::Uuid;

    use super::Station;

    const CONSUMER_GROUP: &str = "mega-test-group";

    static INIT: Once = Once::new();

    fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .pretty()
                .init();
        });
    }

    // static KAFKA_CONTAINER: Lazy<OnceCell<ContainerAsync<GenericImage>>> = Lazy::new(OnceCell::new);

    async fn kafka_container(mapping_port: u16) -> ContainerAsync<GenericImage> {
        // KAFKA_CONTAINER
        //     .get_or_init(|| async {
        GenericImage::new("bitnami/kafka", "3.5")
            .with_wait_for(WaitFor::message_on_stdout("Kafka Server started"))
            .with_mapped_port(mapping_port, 9092.tcp())
            .with_env_var(
                "KAFKA_CFG_LISTENERS",
                "PLAINTEXT://:9092,CONTROLLER://:9093",
            )
            .with_env_var("KAFKA_CFG_CONTROLLER_QUORUM_VOTERS", "1@127.0.0.1:9093")
            .with_env_var("ALLOW_PLAINTEXT_LISTENER", "yes")
            .with_env_var("KAFKA_KRAFT_CLUSTER_ID", "zCG7EfxhRg6MgefynF9sEw==")
            .with_env_var("KAFKA_CFG_NODE_ID", "1")
            .with_env_var("KAFKA_CFG_PROCESS_ROLES", "broker,controller")
            .with_env_var("KAFKA_CFG_CONTROLLER_LISTENER_NAMES", "CONTROLLER")
            .with_env_var(
                "KAFKA_CFG_ADVERTISED_LISTENERS",
                format!("PLAINTEXT://localhost:{}", mapping_port),
            )
            .with_env_var(
                "KAFKA_CFG_LISTENER_SECURITY_PROTOCOL_MAP",
                "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT",
            )
            .start()
            .await
            .expect("Failed to start kafka")
        // })
        // .await
    }

    pub async fn kafka_bootstrap_servers(
        mapping_port: u16,
    ) -> (ContainerAsync<GenericImage>, String) {
        let container = kafka_container(mapping_port).await;
        let port = container.get_host_port_ipv4(9092).await.unwrap();
        (container, format!("localhost:{}", port))
    }

    #[tokio::test]
    pub async fn test_loop_consume() {
        let topic = "test-topic-test_loop_consume".to_owned();
        init_tracing();
        let (container, broker) = kafka_bootstrap_servers(55001).await;

        let satellite = Satellite::new(&broker, &topic);
        assert!(satellite.send_message(&new_crate_msg()).await.is_ok());

        let telescope = Telescope::new(&broker, CONSUMER_GROUP, &topic);
        tokio::select! {
            _ = telescope.consume_loop(|payload| async move {
                println!("✅ test_loop_consume: Received: {}", payload);
            }) => {},
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                println!("⏰ Timeout");
            }
        }
        println!(
            "Release container: {:?}",
            container.get_bridge_ip_address().await
        )
    }

    #[tokio::test]
    pub async fn test_consume_once() {
        let topic = "test-topic-test_consume_once".to_owned();
        init_tracing();
        let (container, broker) = kafka_bootstrap_servers(55002).await;

        let satellite = Satellite::new(&broker, &topic);
        assert!(satellite.send_message(&new_crate_msg()).await.is_ok());

        let telescope = Telescope::new(&broker, CONSUMER_GROUP, &topic);
        let res = telescope
            .consume_once(|msg| println!("✅ test_consume_once: Consume: {}", msg))
            .await;
        println!("{:?}", res);
        assert!(res.is_ok());
        println!(
            "Release container: {:?}",
            container.get_bridge_ip_address().await
        )
    }

    #[tokio::test]
    pub async fn test_station_process() {
        let crate_topic = "test-topic-crate".to_owned();
        init_tracing();
        let (container, broker) = kafka_bootstrap_servers(55003).await;
        let broker = Arc::new(broker);
        let crate_satellite = Satellite::new(&broker, &crate_topic);
        assert!(crate_satellite.send_message(&new_crate_msg()).await.is_ok());

        let broker1 = Arc::clone(&broker);
        let task1 = tokio::spawn(async move {
            let station = Station::new(
                &broker1,
                "test-topic-crate_repo",
                &crate_topic,
                CONSUMER_GROUP,
            );

            tokio::select! {
                _ = station.message_process(|payload| {
                    let crate_message = serde_json::from_str::<CrateMessage>(&payload).unwrap();
                    let mut crate_repo: CrateRepoMessage = crate_message.into();
                    crate_repo.clone_url = "https://localhost:8000".to_owned();
                    serde_json::to_string(&crate_repo).unwrap()
                }) => {},
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    println!("⏰ Timeout");
                }
            }
        });

        task1.await.unwrap();
        let broker2 = Arc::clone(&broker);
        let task2 = tokio::spawn(async move {
            let crate_repo_telescope =
                Telescope::new(&broker2, CONSUMER_GROUP, "test-topic-crate_repo");

            tokio::select! {
                _ = crate_repo_telescope.consume_loop(|payload| async move {
                    println!("✅ Received: {}", payload);
                }) => {},
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    println!("⏰ Timeout");
                }
            }
        });
        task2.await.unwrap();
        println!(
            "Release container: {:?}",
            container.get_bridge_ip_address().await
        )
    }

    fn new_crate_msg() -> String {
        let msg = CrateMessage {
            crate_name: "test".to_owned(),
            crate_version: "1.0.0".to_owned(),
            cksum: "cksum".to_owned(),
            data_source: DataSource::Freighter,
            timestamp: Utc::now(),
            version: "0.0.1".to_owned(),
            uuid: Uuid::new_v4().to_string(),
        };
        serde_json::to_string(&msg).unwrap()
    }
}
