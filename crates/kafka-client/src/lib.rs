// SPDX-License-Identifier: MIT

//! The Kafka client side every Rust port's consumers share, on rdkafka
//! (librdkafka): the consume loop ([`consumer::EventConsumer`], its
//! broker-facing half in `stream`), the dead-letter publisher
//! ([`publisher::KafkaPublisher`]) and the statistics context that feeds the
//! lag gauge ([`context::LagContext`]). What each record goes through
//! (envelope routing, the DLQ guard, age, lag, the consume span) is
//! `kafka-event-consumer`'s, which stays client-free. Moved from
//! user-service-rs's events-probe (gluon `TECHNICAL_DEBT.md` TD-S5).

pub mod consumer;
pub mod context;
pub mod headers;
pub mod publisher;
pub mod settings;
mod stream;

/// The package version, as built.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
