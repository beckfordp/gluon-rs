// SPDX-License-Identifier: MIT

//! purekafka's `DeadLetter`: processing retried with bounded backoff; when
//! the retries run out, the record's own key and value go to `<topic>-dlq`
//! (that publish guarded by purekafka's `Resilience`), are counted in
//! `purekafka.dlq.published`, and the stream carries on. A failed publish
//! fails the guard, so the supervisor restarts the stream.

use std::fmt::Display;
use std::future::Future;

use telemetry::logging::Logger;
use telemetry::metrics::{Counter, Metrics};
use telemetry::resilience::{CircuitBreaker, CircuitBreakerConfig, Retry, RetryConfig};

/// Where dead letters go (a Kafka producer; a fake in tests).
pub trait Publish {
    /// Why a publish failed.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Publishes `value` with `key` to `topic`.
    fn publish(
        &self,
        topic: &str,
        key: Option<&str>,
        value: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Dead-lettering failed, so the record can't be passed over: the stream must
/// restart. Each reads as purekafka's did.
#[derive(Debug, thiserror::Error)]
pub enum DeadLetterError {
    /// The DLQ publish's breaker is open: not even tried.
    #[error("Circuit breaker is open")]
    BreakerOpen,
    /// The DLQ publish failed, after its retries: the publisher's error.
    #[error(transparent)]
    Publish(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// The guard.
pub struct DeadLetter<P> {
    processing: Retry,
    publish_retry: Retry,
    publish_breaker: CircuitBreaker,
    publisher: P,
    published: Counter,
    logger: Logger,
}

impl<P: Publish> DeadLetter<P> {
    /// `dlq_retry` for processing; `publish_retry` and `publish_breaker` for
    /// the DLQ publish (notification-service's `dlq-publish-resilience`).
    #[must_use]
    pub fn new(
        dlq_retry: RetryConfig,
        publish_retry: RetryConfig,
        publish_breaker: CircuitBreakerConfig,
        publisher: P,
        metrics: &Metrics,
        logger: Logger,
    ) -> Self {
        Self {
            processing: Retry::new(dlq_retry, metrics, "purekafka"),
            publish_retry: Retry::new(publish_retry, metrics, "purekafka"),
            publish_breaker: CircuitBreaker::new(publish_breaker, metrics, "purekafka"),
            publisher,
            published: metrics.counter("purekafka.dlq.published"),
            logger,
        }
    }

    async fn publish_once(
        &self,
        topic: &str,
        key: Option<&str>,
        value: &str,
    ) -> Result<(), DeadLetterError> {
        if !self.publish_breaker.acquire() {
            return Err(DeadLetterError::BreakerOpen);
        }
        let result = self.publisher.publish(topic, key, value).await;
        self.publish_breaker.record(result.is_ok());
        result.map_err(|error| DeadLetterError::Publish(Box::new(error)))
    }

    /// Runs `process` (again, up to the retries); routes `(key, value)` to
    /// `<topic>-dlq` if it never succeeds.
    ///
    /// # Errors
    /// Only when the DLQ publish itself fails.
    pub async fn guard<F, Fut, E>(
        &self,
        topic: &str,
        key: Option<&str>,
        value: &str,
        process: F,
    ) -> Result<(), DeadLetterError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<(), E>>,
        E: Display,
    {
        let Err(error) = self
            .processing
            .run(process, Result::is_err, Result::is_err)
            .await
        else {
            return Ok(());
        };
        let dlq_topic = format!("{topic}-dlq");
        self.publish_retry
            .run(
                || self.publish_once(&dlq_topic, key, value),
                |result| matches!(result, Err(DeadLetterError::Publish(_))),
                Result::is_err,
            )
            .await?;
        self.published.add(1, &[]);
        self.logger.error(
            &[
                ("topic", topic),
                ("dlq_topic", &dlq_topic),
                ("key", key.unwrap_or("")),
                ("error", &error.to_string()),
            ],
            "Routed unprocessable message to DLQ after exhausting retries",
        );
        Ok(())
    }
}
