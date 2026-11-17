// SPDX-License-Identifier: MIT

//! One CONSUMER span per record, joining its `traceparent`
//! (purekafka `KafkaTracing.traced`).

use opentelemetry::propagation::{Extractor, TextMapPropagator};
use opentelemetry::trace::{SpanKind, TraceContextExt, Tracer};
use opentelemetry::{Context, KeyValue};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use telemetry::tracing::SdkTracer;

struct Headers<'a>(&'a [(String, String)]);

impl Extractor for Headers<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.iter().map(|(k, _)| k.as_str()).collect()
    }
}

/// A CONSUMER span `<topic> process`, joining the record's W3C context if it
/// carries one, else a new root.
#[must_use]
pub fn consumer_span(tracer: &SdkTracer, headers: &[(String, String)], topic: &str) -> Context {
    let parent = TraceContextPropagator::new().extract(&Headers(headers));
    let span = tracer
        .span_builder(format!("{topic} process"))
        .with_kind(SpanKind::Consumer)
        .with_attributes([
            KeyValue::new("messaging.system", "kafka"),
            KeyValue::new("messaging.destination.name", topic.to_owned()),
        ])
        .start_with_context(tracer, &parent);
    parent.with_span(span)
}
