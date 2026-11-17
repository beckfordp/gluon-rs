// SPDX-License-Identifier: MIT

//! Event age and lag (purekafka `EventAge`, `ConsumerLag`), as the
//! instruments render them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kafka_event_consumer::lag::{ConsumerMetrics, event_age_seconds, lag_from_statistics};
use serde_json::json;
use telemetry::metrics::Metrics;

fn statistics() -> String {
    json!({
        "topics": {
            "user-events": {"partitions": {
                "0": {"partition": 0, "consumer_lag": 3},
                "1": {"partition": 1, "consumer_lag": -1},
                "-1": {"partition": -1, "consumer_lag": -1}
            }},
            "other": {"partitions": {"2": {"partition": 2, "consumer_lag": 0}}}
        }
    })
    .to_string()
}

#[test]
fn event_age_is_now_minus_the_record_time_never_negative() {
    assert!((event_age_seconds(10_500, 10_000) - 0.5).abs() < f64::EPSILON);
    assert!(event_age_seconds(10_000, 12_000).abs() < f64::EPSILON);
}

#[test]
fn lag_comes_from_librdkafkas_statistics_per_assigned_partition() {
    let mut lag = lag_from_statistics(&statistics());
    lag.sort();
    assert_eq!(
        lag,
        [("other".to_owned(), 2, 0), ("user-events".to_owned(), 0, 3)]
    );
    assert_eq!(lag_from_statistics("not json"), []);
    assert_eq!(lag_from_statistics("{}"), []);
}

#[test]
fn age_is_a_histogram_per_topic_and_group() {
    let metrics = Metrics::new("kafka-event-consumer");
    ConsumerMetrics::new(&metrics, "notification-service").record_age(
        "user-events",
        10_500,
        10_000,
    );
    let text = metrics.render();
    assert!(
        text.contains("purekafka_consumer_event_age_seconds_bucket{messaging_consumer_group_name=\"notification-service\",messaging_destination_name=\"user-events\",otel_scope_name=\"kafka-event-consumer\",le=\"0.5\"} 1"),
        "{text}"
    );
    assert!(text.contains("le=\"600.0\""), "{text}");
}

#[test]
fn lag_is_a_gauge_per_assigned_partition() {
    let metrics = Metrics::new("kafka-event-consumer");
    ConsumerMetrics::new(&metrics, "notification-service").record_lag(&statistics());
    let text = metrics.render();
    assert!(
        text.contains("purekafka_consumer_lag{messaging_consumer_group_name=\"notification-service\",messaging_destination_name=\"user-events\",messaging_kafka_destination_partition=\"0\",otel_scope_name=\"kafka-event-consumer\"} 3"),
        "{text}"
    );
    assert!(!text.contains("partition=\"1\""), "{text}");
}
