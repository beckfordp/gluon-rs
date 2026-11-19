// SPDX-License-Identifier: MIT

//! Dead letters go to Kafka through this producer (purekafka `DeadLetter`'s
//! producer): the record's own key and value, no headers.

use std::time::Duration;

use kafka_event_consumer::dead_letter::Publish;
use rdkafka::ClientConfig;
use rdkafka::error::KafkaError;
use rdkafka::message::OwnedHeaders;
use rdkafka::producer::{FutureProducer, FutureRecord};

/// How long a dead letter may wait to be delivered.
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(10);

/// The dead-letter producer.
#[derive(Clone)]
pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    /// A producer on `bootstrap`.
    ///
    /// # Errors
    /// When librdkafka rejects the configuration.
    pub fn new(bootstrap: &str) -> Result<Self, KafkaError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set(
                "message.timeout.ms",
                DELIVERY_TIMEOUT.as_millis().to_string(),
            )
            .create()?;
        Ok(Self { producer })
    }
}

impl Publish for KafkaPublisher {
    type Error = KafkaError;

    async fn publish(&self, topic: &str, key: Option<&str>, value: &str) -> Result<(), KafkaError> {
        let mut record = FutureRecord::to(topic)
            .payload(value)
            .headers(OwnedHeaders::new());
        if let Some(key) = key {
            record = record.key(key);
        }
        self.producer
            .send(record, DELIVERY_TIMEOUT)
            .await
            .map(|_| ())
            .map_err(|(error, _)| error)
    }
}
