// SPDX-License-Identifier: MIT

//! The consume span (purekafka `KafkaTracing.traced`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kafka_event_consumer::span::consumer_span;
use opentelemetry::trace::{SpanKind, TraceContextExt};
use opentelemetry_sdk::trace::InMemorySpanExporter;
use telemetry::tracing::Tracing;

fn finished(headers: &[(String, String)]) -> opentelemetry_sdk::trace::SpanData {
    let exporter = InMemorySpanExporter::default();
    let tracing = Tracing::with_exporter("kafka-event-consumer", 1.0, exporter.clone());
    let cx = consumer_span(&tracing.tracer(), headers, "user-events");
    cx.span().end();
    tracing.force_flush();
    exporter.get_finished_spans().unwrap().remove(0)
}

#[test]
fn each_record_gets_a_consumer_span_joining_its_traceparent() {
    let span = finished(&[(
        "Traceparent".to_owned(),
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_owned(),
    )]);
    assert_eq!(span.name, "user-events process");
    assert_eq!(span.span_kind, SpanKind::Consumer);
    assert_eq!(
        span.span_context.trace_id().to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736"
    );
    assert_eq!(span.parent_span_id.to_string(), "00f067aa0ba902b7");
    let attributes: Vec<(String, String)> = span
        .attributes
        .iter()
        .map(|kv| (kv.key.to_string(), kv.value.to_string()))
        .collect();
    assert_eq!(
        attributes,
        [
            ("messaging.system".to_owned(), "kafka".to_owned()),
            (
                "messaging.destination.name".to_owned(),
                "user-events".to_owned()
            )
        ]
    );
}

#[test]
fn a_record_without_a_traceparent_starts_a_new_trace() {
    let span = finished(&[("other".to_owned(), "x".to_owned())]);
    assert_eq!(span.parent_span_id.to_string(), "0000000000000000");
}
