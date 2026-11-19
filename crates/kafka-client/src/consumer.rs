// SPDX-License-Identifier: MIT

//! One consumer: what it needs, and what it does with each record. The loop
//! that feeds it records from the broker is [`EventConsumer::consume`].

use opentelemetry::context::FutureExt;
use opentelemetry::trace::TraceContextExt;
use telemetry::logging::Logger;
use telemetry::metrics::Metrics;
use telemetry::tracing::SdkTracer;

use kafka_event_consumer::dead_letter::{DeadLetter, DeadLetterError, Publish};
use kafka_event_consumer::envelope::{Routes, handle_record};
use kafka_event_consumer::span::consumer_span;
use rdkafka::error::KafkaError;

use crate::settings::ConsumerSettings;

/// Why a consumer stream failed, so the supervisor rebuilds it. Each reads as
/// the message it replaced (gluon TD-64.1), the cause kept as its source.
#[derive(Debug, thiserror::Error)]
pub enum ConsumeError {
    /// librdkafka refused the configuration or the subscription.
    #[error(transparent)]
    Subscribe(KafkaError),
    /// Every broker is gone.
    #[error(transparent)]
    AllBrokersDown(KafkaError),
    /// A handled record's offset wasn't committed.
    #[error("commit failed: {0}")]
    Commit(#[source] KafkaError),
    /// A record couldn't even be dead-lettered.
    #[error(transparent)]
    DeadLetter(#[from] DeadLetterError),
}

/// A consumer: its settings, telemetry, routes and DLQ guard.
pub struct EventConsumer<R, P> {
    /// Broker, group, topic, lag interval.
    pub settings: ConsumerSettings,
    /// Spans.
    pub tracer: SdkTracer,
    /// JSON log lines.
    pub logger: Logger,
    /// Where metrics go.
    pub metrics: Metrics,
    /// What the consumer does with each event type.
    pub routes: R,
    /// The DLQ guard.
    pub dead_letter: DeadLetter<P>,
}

impl<R: Routes, P: Publish + Sync> EventConsumer<R, P> {
    /// Handles one record inside its "<topic> process" span (gluon-rs
    /// `kafka-event-consumer`'s `handle_record`: a null value or unrelated type
    /// is skipped; anything that fails, poison included, is retried and then
    /// dead-lettered raw). A null key is the key `""`, as Scala's consumers
    /// hand the guard `record.key.getOrElse("")` (gluon TECHNICAL-DEBT.md S-4).
    ///
    /// # Errors
    /// Only when dead-lettering failed too: the stream must restart.
    pub async fn process(
        &self,
        topic: &str,
        key: Option<&str>,
        value: Option<&str>,
        headers: &[(String, String)],
    ) -> Result<(), DeadLetterError> {
        let cx = consumer_span(&self.tracer, headers, topic);
        let result = handle_record(
            topic,
            Some(key.unwrap_or_default()),
            value,
            &self.routes,
            &self.logger,
            &self.dead_letter,
        )
        .with_context(cx.clone())
        .await;
        cx.span().end();
        result
    }
}
