// SPDX-License-Identifier: MIT

//! The per-record decision notification-service's consumers share
//! (`EnvelopeHandler`): a null value is skipped; an envelope or payload that
//! doesn't decode is logged once, then dead-lettered raw after the guard's
//! retries; an event type the consumer has no route for is skipped; a routed
//! event is handled inside the DLQ guard. A consumer says what it routes and
//! what it does through [`Routes`].

use std::future::Future;

use serde::Deserialize;
use serde_json::Value;
use telemetry::logging::Logger;
use time::OffsetDateTime;

use crate::dead_letter::{DeadLetter, Publish};

/// The shared envelope (gluon docs/system-design.md "Envelope").
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// Unique per event: consumers' idempotency key.
    pub event_id: String,
    /// What happened, e.g. `UserRegistered`: the routing key.
    pub event_type: String,
    /// When: an instant, as Scala's `EventEnvelope` decodes it (RFC 3339, as
    /// `java.time.Instant` writes it); an envelope whose timestamp isn't one
    /// doesn't decode, so it is poison (gluon TECHNICAL-DEBT.md S-3).
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    /// The producing service.
    pub source: String,
    /// Ties the event to its request.
    pub correlation_id: String,
    /// The event itself, decoded by the route.
    pub payload: Value,
}

/// A consumer's routes: which event types it acts on, and what it does.
pub trait Routes: Sync {
    /// The decoded events (an enum when a consumer routes several types).
    type Event: Send + Sync;

    /// `None` when the envelope's type has no route (skipped); `Some(Err)`
    /// when it has one but the payload doesn't decode (poison).
    fn decode(&self, envelope: &Envelope) -> Option<Result<Self::Event, String>>;

    /// Acts on `event`; run inside the DLQ guard, so an `Err` is retried and
    /// then dead-lettered.
    fn handle(
        &self,
        event_id: &str,
        event: &Self::Event,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// Handles one record's value.
///
/// # Errors
/// Only when dead-lettering failed too: the stream must restart.
pub async fn handle_record<R: Routes, P: Publish + Sync>(
    topic: &str,
    key: Option<&str>,
    value: Option<&str>,
    routes: &R,
    logger: &Logger,
    dead_letter: &DeadLetter<P>,
) -> Result<(), String> {
    let Some(raw) = value else {
        logger.error(
            &[("topic", topic)],
            &format!("Received {topic} record with a null value - skipping"),
        );
        return Ok(());
    };
    let envelope: Envelope = match serde_json::from_str(raw) {
        Ok(envelope) => envelope,
        Err(error) => {
            let error = error.to_string();
            let bytes = raw.len().to_string();
            logger.error(
                &[("topic", topic), ("bytes", &bytes), ("error", &error)],
                &format!("Failed to decode {topic} envelope"),
            );
            return poison(topic, key, raw, dead_letter, &error).await;
        }
    };
    match routes.decode(&envelope) {
        None => {
            logger.info(
                &[("topic", topic), ("event_type", &envelope.event_type)],
                &format!("Ignoring unrelated {topic} event type"),
            );
            Ok(())
        }
        Some(Err(error)) => {
            logger.error(
                &[
                    ("topic", topic),
                    ("event_type", &envelope.event_type),
                    ("error", &error),
                ],
                &format!("Failed to decode {} payload", envelope.event_type),
            );
            poison(topic, key, raw, dead_letter, &error).await
        }
        Some(Ok(event)) => {
            dead_letter
                .guard(topic, key, raw, || {
                    routes.handle(&envelope.event_id, &event)
                })
                .await
        }
    }
}

/// A record no retry will fix still goes through the guard, so it is retried
/// and dead-lettered as any failure is (`DeadLetter.guard` around the
/// decode in `EnvelopeHandler`).
async fn poison<P: Publish + Sync>(
    topic: &str,
    key: Option<&str>,
    raw: &str,
    dead_letter: &DeadLetter<P>,
    error: &str,
) -> Result<(), String> {
    dead_letter
        .guard(topic, key, raw, || {
            std::future::ready(Err(error.to_owned()))
        })
        .await
}
