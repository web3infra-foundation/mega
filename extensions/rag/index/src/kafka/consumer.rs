use rdkafka::config::ClientConfig;
use rdkafka::consumer::CommitMode;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    Message,
};
use tokio_stream::StreamExt;

pub fn new_consumer(brokers: &str, group_id: &str) -> StreamConsumer {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation error")
}

pub async fn consume_messages(consumer: StreamConsumer) {
    let mut message_stream = consumer.stream();

    while let Some(message) = message_stream.next().await {
        match message {
            Ok(m) => match m.payload_view::<str>() {
                Some(Ok(payload)) => {
                    println!("Received: {}", payload);
                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
                Some(Err(e)) => eprintln!("UTF-8 error: {}", e),
                None => println!("Empty message"),
            },
            Err(e) => eprintln!("Kafka error: {}", e),
        }
    }
}
