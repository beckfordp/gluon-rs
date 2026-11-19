// SPDX-License-Identifier: MIT

//! One consumer, assembled and supervised, so a service states only its
//! settings and routes (gluon TECHNICAL-DEBT.md S-1): the DLQ producer
//! ([`KafkaPublisher`]), purekafka's `DeadLetter` guard with its processing
//! retry and its publish's retry and breaker, the consume loop
//! ([`EventConsumer`]), and `StreamSupervisor`'s restart with backoff
//! (`kafka-event-consumer`'s `supervise`). Moved from the events-probe's and
//! notification-service-rs's hand-built copies.

use std::time::Duration;

use kafka_event_consumer::dead_letter::DeadLetter;
use kafka_event_consumer::envelope::Routes;
use kafka_event_consumer::supervisor::supervise;
use rdkafka::error::KafkaError;
use telemetry::logging::Logger;
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};
use telemetry::tracing::SdkTracer;
use tokio::sync::watch;

use crate::consumer::EventConsumer;
use crate::publisher::KafkaPublisher;
use crate::settings::ConsumerSettings;

/// purekafka's consumer-side resilience. [`Default`] is notification-service's
/// `application.conf` `kafka.*` (the probe's defaults too).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resilience {
    /// `dlq-retry`: a record's processing, retried before it is dead-lettered.
    pub dlq_retry: RetryConfig,
    /// `dlq-publish-resilience.retry`: the dead letter's publish.
    pub dlq_publish_retry: RetryConfig,
    /// `dlq-publish-resilience.circuit-breaker`.
    pub dlq_publish_breaker: CircuitBreakerConfig,
    /// `stream-supervisor.base-delay`: the first restart's delay, doubling...
    pub restart_base_delay: Duration,
    /// ... to `stream-supervisor.max-delay`.
    pub restart_max_delay: Duration,
}

impl Default for Resilience {
    fn default() -> Self {
        Self {
            dlq_retry: RetryConfig {
                max_retries: 3,
                base_delay: Duration::from_millis(200),
            },
            dlq_publish_retry: RetryConfig {
                max_retries: 3,
                base_delay: Duration::from_millis(100),
            },
            dlq_publish_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: Duration::from_secs(30),
            },
            restart_base_delay: Duration::from_secs(1),
            restart_max_delay: Duration::from_secs(30),
        }
    }
}

/// What a consumer logs, traces and measures with.
#[derive(Clone)]
pub struct ConsumerTelemetry {
    /// One CONSUMER span per record.
    pub tracer: SdkTracer,
    /// JSON log lines.
    pub logger: Logger,
    /// Event age, lag, DLQ and restart counts.
    pub metrics: Metrics,
}

/// Runs one consumer of `settings` with `routes` until `stop` is set: each
/// record handled under the DLQ guard and committed after; a failed stream
/// rebuilt after a backoff, forever.
///
/// # Errors
/// When the DLQ producer can't be built from `settings.bootstrap`.
pub async fn run_supervised<R: Routes + Send>(
    settings: ConsumerSettings,
    resilience: &Resilience,
    routes: R,
    telemetry: ConsumerTelemetry,
    stop: watch::Receiver<bool>,
) -> Result<(), KafkaError> {
    let ConsumerTelemetry {
        tracer,
        logger,
        metrics,
    } = telemetry;
    let publisher = KafkaPublisher::new(&settings.bootstrap)?;
    let dead_letter = DeadLetter::new(
        resilience.dlq_retry,
        resilience.dlq_publish_retry,
        resilience.dlq_publish_breaker,
        publisher,
        &metrics,
        logger.clone(),
    );
    let consumer = EventConsumer {
        settings,
        tracer,
        logger,
        metrics,
        routes,
        dead_letter,
    };
    supervise(
        resilience.restart_base_delay,
        resilience.restart_max_delay,
        &consumer.metrics,
        &consumer.logger,
        stop.clone(),
        || consumer.consume(stop.clone()),
    )
    .await;
    Ok(())
}
