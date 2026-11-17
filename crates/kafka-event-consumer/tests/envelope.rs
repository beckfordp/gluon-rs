// SPDX-License-Identifier: MIT

//! The per-record decision (notification-service `EnvelopeHandler`) through a
//! fake consumer's routes, a fake DLQ publisher and a log buffer: null and
//! unrelated records skipped, poison logged once and dead-lettered raw, a
//! routed event handled with its `eventId` under the guard.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unused_async_trait_impl
)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use kafka_event_consumer::dead_letter::{DeadLetter, Publish};
use kafka_event_consumer::envelope::{Envelope, Routes, handle_record};
use serde::Deserialize;
use serde_json::{Value, json};
use telemetry::logging::Logger;
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};

/// A dead letter: topic, key, value.
type Letter = (String, Option<String>, String);

#[derive(Clone, Default)]
struct Published {
    records: Arc<Mutex<Vec<Letter>>>,
    fail: bool,
}

impl Publish for Published {
    async fn publish(&self, topic: &str, key: Option<&str>, value: &str) -> Result<(), String> {
        if self.fail {
            return Err("broker gone".to_owned());
        }
        self.records.lock().unwrap().push((
            topic.to_owned(),
            key.map(ToOwned::to_owned),
            value.to_owned(),
        ));
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Welcome {
    user_id: String,
}

/// Routes `UserRegistered` only; records each handled `(eventId, userId)`
/// and fails each call while `failing`.
#[derive(Default)]
struct Fake {
    handled: Mutex<Vec<(String, String)>>,
    failing: bool,
}

impl Routes for Fake {
    type Event = Welcome;

    fn decode(&self, envelope: &Envelope) -> Option<Result<Welcome, String>> {
        (envelope.event_type == "UserRegistered").then(|| {
            serde_json::from_value(envelope.payload.clone()).map_err(|error| error.to_string())
        })
    }

    async fn handle(&self, event_id: &str, event: &Welcome) -> Result<(), String> {
        self.handled
            .lock()
            .unwrap()
            .push((event_id.to_owned(), event.user_id.clone()));
        if self.failing {
            Err("lookup failed".to_owned())
        } else {
            Ok(())
        }
    }
}

struct Run {
    result: Result<(), String>,
    letters: Vec<Letter>,
    lines: Vec<Value>,
}

async fn run(routes: &Fake, publisher: Published, key: Option<&str>, value: Option<&str>) -> Run {
    let log: Arc<Mutex<Vec<u8>>> = Arc::default();
    let logger = Logger::to_writer("kafka-event-consumer", log.clone());
    let retry = RetryConfig {
        max_retries: 1,
        base_delay: Duration::from_millis(1),
    };
    let dead_letter = DeadLetter::new(
        retry,
        retry,
        CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
        },
        publisher.clone(),
        &Metrics::new("kafka-event-consumer"),
        logger.clone(),
    );
    let result = handle_record("user-events", key, value, routes, &logger, &dead_letter).await;
    let lines = String::from_utf8(log.lock().unwrap().clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let letters = publisher.records.lock().unwrap().clone();
    Run {
        result,
        letters,
        lines,
    }
}

fn envelope(event_type: &str, payload: &Value) -> String {
    json!({
        "eventId": "e1", "eventType": event_type, "timestamp": "2026-11-09T18:43:19.487722Z",
        "source": "user-service", "correlationId": "c1", "payload": payload
    })
    .to_string()
}

fn messages(lines: &[Value]) -> Vec<&str> {
    lines
        .iter()
        .map(|line| line["message"].as_str().unwrap())
        .collect()
}

const DLQ_LINE: &str = "Routed unprocessable message to DLQ after exhausting retries";

#[tokio::test]
async fn a_routed_event_is_handled_with_its_event_id() {
    let routes = Fake::default();
    let value = envelope("UserRegistered", &json!({"userId": "u1"}));
    let run = run(&routes, Published::default(), Some("u1"), Some(&value)).await;
    assert_eq!(run.result, Ok(()));
    assert_eq!(
        *routes.handled.lock().unwrap(),
        [("e1".to_owned(), "u1".to_owned())]
    );
    assert_eq!(run.letters, []);
    assert!(run.lines.is_empty(), "{:?}", run.lines);
}

#[tokio::test]
async fn a_null_value_is_skipped_with_an_error_line() {
    let routes = Fake::default();
    let run = run(&routes, Published::default(), Some("k"), None).await;
    assert_eq!(run.result, Ok(()));
    assert_eq!(run.letters, []);
    assert_eq!(run.lines.len(), 1);
    assert_eq!(run.lines[0]["level"], "ERROR");
    assert_eq!(
        run.lines[0]["message"],
        "Received user-events record with a null value - skipping"
    );
}

#[tokio::test]
async fn an_unrelated_type_is_skipped_without_handling() {
    let routes = Fake::default();
    let value = envelope("UserDeleted", &json!({"userId": "u1"}));
    let run = run(&routes, Published::default(), None, Some(&value)).await;
    assert_eq!(run.result, Ok(()));
    assert!(routes.handled.lock().unwrap().is_empty());
    assert_eq!(run.letters, []);
    assert_eq!(run.lines[0]["level"], "INFO");
    assert_eq!(
        run.lines[0]["message"],
        "Ignoring unrelated user-events event type"
    );
    assert_eq!(run.lines[0]["event_type"], "UserDeleted");
}

#[tokio::test]
async fn an_undecodable_envelope_is_logged_once_and_dead_lettered_raw() {
    for raw in ["{nope", &json!({"eventType": "UserRegistered"}).to_string()] {
        let run = run(
            &Fake::default(),
            Published::default(),
            Some("k1"),
            Some(raw),
        )
        .await;
        assert_eq!(run.result, Ok(()));
        assert_eq!(
            run.letters,
            [(
                "user-events-dlq".to_owned(),
                Some("k1".to_owned()),
                raw.to_owned()
            )]
        );
        assert_eq!(
            messages(&run.lines),
            ["Failed to decode user-events envelope", DLQ_LINE]
        );
        assert_eq!(run.lines[0]["bytes"], raw.len().to_string());
    }
}

#[tokio::test]
async fn an_undecodable_payload_is_logged_once_and_dead_lettered_raw() {
    let routes = Fake::default();
    let value = envelope("UserRegistered", &json!({"email": "no user id"}));
    let run = run(&routes, Published::default(), None, Some(&value)).await;
    assert_eq!(run.result, Ok(()));
    assert!(routes.handled.lock().unwrap().is_empty());
    assert_eq!(run.letters.len(), 1);
    assert_eq!(run.letters[0].2, value);
    assert_eq!(
        messages(&run.lines),
        ["Failed to decode UserRegistered payload", DLQ_LINE]
    );
}

#[tokio::test]
async fn a_failing_handler_is_retried_then_dead_lettered() {
    let routes = Fake {
        failing: true,
        ..Fake::default()
    };
    let value = envelope("UserRegistered", &json!({"userId": "u1"}));
    let run = run(&routes, Published::default(), Some("u1"), Some(&value)).await;
    assert_eq!(run.result, Ok(()));
    assert_eq!(
        routes.handled.lock().unwrap().len(),
        2,
        "first try and one retry"
    );
    assert_eq!(run.letters.len(), 1);
    assert_eq!(messages(&run.lines), [DLQ_LINE]);
}

#[tokio::test]
async fn a_failed_dead_letter_is_the_streams_error() {
    let publisher = Published {
        fail: true,
        ..Published::default()
    };
    let run = run(&Fake::default(), publisher, None, Some("{nope")).await;
    assert!(run.result.is_err());
}
