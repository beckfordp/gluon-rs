// SPDX-License-Identifier: MIT

//! The transactional outbox (ADR 0008) as the Scala services write it: the
//! complete event envelope (`eventId`, `eventType`, `timestamp`, `source`,
//! `correlationId`, `payload`, in that order) in `outbox_events.payload`, in
//! the same transaction as the change it announces; Debezium's `EventRouter`
//! publishes it keyed by `aggregate_id`, with `trace_parent` and
//! `correlation_id` as headers. Moved from user-service-rs for the all-Rust
//! ports (gluon Stage A P0.3).

use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::jvm::java_instant;

/// The table, as every Scala service's `V*__create_outbox_events.sql` makes it.
pub const MIGRATION: &str = "CREATE TABLE outbox_events (
    id UUID PRIMARY KEY,
    aggregate_id TEXT NOT NULL,
    payload JSONB NOT NULL,
    trace_parent TEXT,
    correlation_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
";

/// An `outbox_events` row, ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEvent {
    /// The row id (not the envelope's `eventId`).
    pub id: Uuid,
    /// The Kafka key.
    pub aggregate_id: String,
    /// The envelope, stored as JSONB and published as it is.
    pub payload: Value,
    /// The envelope's `correlationId`, also a Kafka header.
    pub correlation_id: String,
}

/// The row for `payload` as an `event_type` from `source`, stamped `now` at
/// the resolution of Java's clock (microseconds), as the Scala services' are.
///
/// # Errors
/// A payload that doesn't serialise to JSON.
pub fn envelope<P: Serialize>(
    event_type: &str,
    source: &str,
    aggregate_id: &str,
    correlation_id: &str,
    payload: &P,
    now: OffsetDateTime,
) -> Result<OutboxEvent, serde_json::Error> {
    let micros = now
        .replace_nanosecond(now.nanosecond() / 1_000 * 1_000)
        .unwrap_or(now);
    let mut envelope = Map::new();
    envelope.insert("eventId".into(), json!(Uuid::new_v4().to_string()));
    envelope.insert("eventType".into(), json!(event_type));
    envelope.insert("timestamp".into(), json!(java_instant(micros)));
    envelope.insert("source".into(), json!(source));
    envelope.insert("correlationId".into(), json!(correlation_id));
    envelope.insert("payload".into(), serde_json::to_value(payload)?);
    Ok(OutboxEvent {
        id: Uuid::new_v4(),
        aggregate_id: aggregate_id.to_owned(),
        payload: Value::Object(envelope),
        correlation_id: correlation_id.to_owned(),
    })
}

/// Writes `event` on `tx` (the caller's transaction), with the W3C
/// `traceparent` of the span it belongs to, if any.
///
/// # Errors
/// Database failures.
pub async fn write(
    tx: &mut PgConnection,
    event: &OutboxEvent,
    trace_parent: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO outbox_events (id, aggregate_id, payload, trace_parent, correlation_id)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(event.id)
    .bind(&event.aggregate_id)
    .bind(&event.payload)
    .bind(trace_parent)
    .bind(&event.correlation_id)
    .execute(tx)
    .await?;
    Ok(())
}
