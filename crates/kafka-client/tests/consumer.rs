// SPDX-License-Identifier: MIT

//! One record through `EventConsumer::process`, without a broker: inside its
//! "<topic> process" span, routed by gluon-rs `kafka-event-consumer` under the
//! DLQ guard (purekafka `KafkaTracing.traced`, `DeadLetter`; notification-service's
//! `EnvelopeHandler`, as moved from user-service-rs's events-probe, gluon TD-S5).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unused_async_trait_impl
)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kafka_client::clock::{Clock, SystemClock};
use kafka_client::consumer::EventConsumer;
use kafka_client::settings::ConsumerSettings;
use kafka_event_consumer::dead_letter::{DeadLetter, Publish};
use kafka_event_consumer::envelope::{Envelope, Routes};
use kafka_event_consumer::lag::ConsumerMetrics;
use opentelemetry_sdk::trace::InMemorySpanExporter;
use serde_json::json;
use telemetry::logging::Logger;
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};
use telemetry::tracing::Tracing;

/// A dead letter: topic, key, value.
type Letter = (String, Option<String>, String);

#[derive(Clone, Default)]
struct Letters(Arc<Mutex<Vec<Letter>>>);

impl Publish for Letters {
    type Error = std::io::Error;

    async fn publish(
        &self,
        topic: &str,
        key: Option<&str>,
        value: &str,
    ) -> Result<(), std::io::Error> {
        self.0.lock().unwrap().push((
            topic.to_owned(),
            key.map(ToOwned::to_owned),
            value.to_owned(),
        ));
        Ok(())
    }
}

/// Routes `Hello`, whose payload must have a `name`; records what it handled.
#[derive(Default)]
struct Hellos(Arc<Mutex<Vec<(String, String)>>>);

impl Routes for Hellos {
    type Event = String;
    type Error = std::io::Error;

    fn decode(&self, envelope: &Envelope) -> Option<Result<String, std::io::Error>> {
        (envelope.event_type == "Hello").then(|| {
            envelope.payload["name"]
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| std::io::Error::other("no name"))
        })
    }

    async fn handle(&self, event_id: &str, event: &String) -> Result<(), std::io::Error> {
        self.0
            .lock()
            .unwrap()
            .push((event_id.to_owned(), event.clone()));
        Ok(())
    }
}

/// A clock stopped at `0` + `millis`.
struct Stopped(u64);

impl Clock for Stopped {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.0)
    }
}

fn consumer(exporter: &InMemorySpanExporter) -> (EventConsumer<Hellos, Letters>, Letters, Tracing) {
    let metrics = Metrics::new("kafka-client");
    let logger = Logger::to_writer("kafka-client", Arc::new(Mutex::new(Vec::<u8>::new())));
    let letters = Letters::default();
    let quick = RetryConfig {
        max_retries: 0,
        base_delay: Duration::from_millis(1),
    };
    let dead_letter = DeadLetter::new(
        quick,
        quick,
        CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
        },
        letters.clone(),
        &metrics,
        logger.clone(),
    );
    let tracing = Tracing::with_exporter("kafka-client", 1.0, exporter.clone());
    let consumer = EventConsumer {
        settings: ConsumerSettings {
            bootstrap: "127.0.0.1:9".to_owned(),
            group_id: "g".to_owned(),
            topic: "hellos".to_owned(),
            lag_interval: Duration::from_secs(5),
        },
        tracer: tracing.tracer(),
        logger,
        metrics,
        routes: Hellos::default(),
        dead_letter,
        clock: Arc::new(Stopped(10_500)),
    };
    (consumer, letters, tracing)
}

fn hello(payload: &serde_json::Value) -> String {
    json!({"eventId": "e1", "eventType": "Hello", "timestamp": "2026-11-17T00:00:00Z",
           "source": "test", "correlationId": "c", "payload": payload})
    .to_string()
}

#[tokio::test]
async fn a_routed_event_is_handled_inside_its_consume_span() {
    let exporter = InMemorySpanExporter::default();
    let (consumer, letters, tracing) = consumer(&exporter);
    let value = hello(&json!({"name": "ada"}));
    let parent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    let headers = vec![("traceparent".to_owned(), parent.to_owned())];
    consumer
        .process("hellos", Some("k"), Some(&value), &headers)
        .await
        .unwrap();
    assert_eq!(
        *consumer.routes.0.lock().unwrap(),
        vec![("e1".to_owned(), "ada".to_owned())]
    );
    assert!(letters.0.lock().unwrap().is_empty());
    tracing.force_flush();
    let spans = exporter.get_finished_spans().unwrap();
    let span = spans.iter().find(|s| s.name == "hellos process").unwrap();
    assert_eq!(
        span.span_context.trace_id().to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736"
    );
}

#[tokio::test]
async fn a_payload_that_does_not_decode_is_dead_lettered_raw() {
    let exporter = InMemorySpanExporter::default();
    let (consumer, letters, _tracing) = consumer(&exporter);
    let value = hello(&json!({}));
    consumer
        .process("hellos", Some("k"), Some(&value), &[])
        .await
        .unwrap();
    assert!(consumer.routes.0.lock().unwrap().is_empty());
    assert_eq!(
        *letters.0.lock().unwrap(),
        vec![("hellos-dlq".to_owned(), Some("k".to_owned()), value)]
    );
}

#[tokio::test]
async fn a_null_key_is_dead_lettered_with_an_empty_key() {
    // Scala's consumers hand the guard record.key.getOrElse(""): a null-key
    // record's dead letter has the key "", not none (gluon TECHNICAL-DEBT.md S-4).
    let exporter = InMemorySpanExporter::default();
    let (consumer, letters, _tracing) = consumer(&exporter);
    consumer
        .process("hellos", None, Some("{not json"), &[])
        .await
        .unwrap();
    assert_eq!(
        *letters.0.lock().unwrap(),
        vec![(
            "hellos-dlq".to_owned(),
            Some(String::new()),
            "{not json".to_owned()
        )]
    );
}

#[test]
fn a_records_age_is_read_from_the_consumers_clock() {
    let exporter = InMemorySpanExporter::default();
    let (consumer, _, _) = consumer(&exporter);
    let ages = ConsumerMetrics::new(&consumer.metrics, "g");
    consumer.record_age(&ages, "hellos", 10_000);
    let text = consumer.metrics.render();
    assert!(
        text.contains("purekafka_consumer_event_age_seconds_bucket{messaging_consumer_group_name=\"g\",messaging_destination_name=\"hellos\",otel_scope_name=\"kafka-client\",le=\"0.5\"} 1"),
        "{text}"
    );
    assert!(
        text.contains("purekafka_consumer_event_age_seconds_bucket{messaging_consumer_group_name=\"g\",messaging_destination_name=\"hellos\",otel_scope_name=\"kafka-client\",le=\"0.25\"} 0"),
        "half a second, not less: {text}"
    );
}

#[test]
fn the_system_clock_is_the_wall_clock() {
    let before = SystemTime::now();
    let now = SystemClock.now();
    assert!(now >= before && now <= SystemTime::now());
}
