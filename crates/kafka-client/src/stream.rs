// SPDX-License-Identifier: MIT

//! The consume loop against the broker: each record timed for age, handled
//! ([`EventConsumer::process`]), then its offset committed (at least once, as
//! fs2-kafka's `commit` after `handle` in notification-service). It fails when
//! every broker is gone or a commit fails; the caller supervises it (gluon-rs
//! `kafka-event-consumer`'s `supervise`). Needs a broker, so the contract
//! harness runs it (the probe group, notification-service's consumer groups),
//! not this crate's tests.

use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::{Headers, Message};
use rdkafka::types::RDKafkaErrorCode;

use tokio::sync::watch;

use kafka_event_consumer::dead_letter::Publish;
use kafka_event_consumer::envelope::Routes;
use kafka_event_consumer::lag::ConsumerMetrics;

use crate::blocking::off_worker;
use crate::consumer::{ConsumeError, EventConsumer};
use crate::context::LagContext;
use crate::headers::text_headers;
use crate::settings::ConsumerSettings;

/// A consumer subscribed to `settings`' topic, its statistics feeding the lag
/// gauge of `metrics` (the consumer's own, built once: gluon TD-64.4).
///
/// # Errors
/// When librdkafka rejects the configuration or the subscription.
pub fn subscribe(
    settings: &ConsumerSettings,
    metrics: ConsumerMetrics,
) -> Result<StreamConsumer<LagContext>, KafkaError> {
    let context = LagContext { metrics };
    let consumer: StreamConsumer<LagContext> =
        settings.client_config().create_with_context(context)?;
    consumer.subscribe(&[settings.topic.as_str()])?;
    Ok(consumer)
}

impl<R: Routes, P: Publish + Sync> EventConsumer<R, P> {
    /// Consumes until `stop` says so: each record's age recorded, the record
    /// processed, its offset committed.
    ///
    /// # Errors
    /// The stream's failure: every broker gone, a failed commit, or a record
    /// that couldn't even be dead-lettered.
    pub async fn consume(&self, mut stop: watch::Receiver<bool>) -> Result<(), ConsumeError> {
        let ages = ConsumerMetrics::new(&self.metrics, self.settings.group_id.as_str());
        let consumer = subscribe(&self.settings, ages.clone()).map_err(ConsumeError::Subscribe)?;
        loop {
            let message = tokio::select! {
                _ = stop.wait_for(|stop| *stop) => return Ok(()),
                message = consumer.recv() => message,
            };
            let message = match message {
                Ok(message) => message,
                Err(error)
                    if error.rdkafka_error_code() == Some(RDKafkaErrorCode::AllBrokersDown) =>
                {
                    return Err(ConsumeError::AllBrokersDown(error));
                }
                Err(error) => {
                    self.logger
                        .warn(&[("error", &error.to_string())], "Consumer error");
                    continue;
                }
            };
            let topic = message.topic().to_owned();
            if let Some(timestamp) = message.timestamp().to_millis() {
                self.record_age(&ages, &topic, timestamp);
            }
            let key = message
                .key()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
            let value = message
                .payload()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
            let headers = message.headers().map_or_else(Vec::new, |headers| {
                text_headers(headers.iter().map(|header| (header.key, header.value)))
            });
            self.process(&topic, key.as_deref(), value.as_deref(), &headers)
                .await?;
            off_worker(|| consumer.commit_message(&message, CommitMode::Sync))
                .map_err(ConsumeError::Commit)?;
        }
    }
}
