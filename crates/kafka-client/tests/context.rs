// SPDX-License-Identifier: MIT

//! librdkafka's statistics become the lag gauge (purekafka `ConsumerLag`;
//! moved with the context from user-service-rs's events-probe, gluon TD-S5).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kafka_client::context::LagContext;
use kafka_event_consumer::lag::ConsumerMetrics;
use rdkafka::ClientContext;
use serde_json::json;
use telemetry::metrics::Metrics;

#[test]
fn librdkafkas_statistics_become_the_lag_gauge() {
    let metrics = Metrics::new("kafka-client");
    let context = LagContext {
        metrics: ConsumerMetrics::new(&metrics, "a-group"),
    };
    let statistics = json!({"topics": {"user-events": {"partitions": {
        "0": {"partition": 0, "consumer_lag": 7}
    }}}})
    .to_string();
    context.stats_raw(statistics.as_bytes());
    let text = metrics.render();
    assert!(text.contains("purekafka_consumer_lag{"), "{text}");
    assert!(text.contains("} 7"), "{text}");
}
