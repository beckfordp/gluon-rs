// SPDX-License-Identifier: MIT

//! The consumer side every Rust port's Kafka consumers share, as purekafka's
//! is for the Scala services: envelope routing (`EnvelopeHandler`), the DLQ
//! guard (`DeadLetter`), the stream supervisor (`StreamSupervisor`), event
//! age and lag (`EventAge`, `ConsumerLag`) and the consume span
//! (`KafkaTracing.traced`). No Kafka client: the caller's consume loop owns
//! the broker, the offsets and the publisher behind [`dead_letter::Publish`].
//! Moved from user-service-rs's events-probe (gluon RPN-8).

pub mod dead_letter;
pub mod envelope;
pub mod lag;
pub mod span;
pub mod supervisor;

/// The package version, as built.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
