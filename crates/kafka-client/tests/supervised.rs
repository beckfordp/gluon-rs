// SPDX-License-Identifier: MIT

//! One consumer, assembled and supervised (gluon TECHNICAL-DEBT.md S-1):
//! purekafka's consumer-side settings as notification-service's
//! `application.conf` sets them, and a consumer told to stop returning cleanly.
//! Consuming needs a broker: tests/contract's consumer groups run that (the
//! probe group, notification-service's user-events group).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unused_async_trait_impl
)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use kafka_client::KafkaError;
use kafka_client::settings::ConsumerSettings;
use kafka_client::supervised::{ConsumerTelemetry, Resilience, run_supervised};
use kafka_event_consumer::envelope::{Envelope, Routes};
use opentelemetry_sdk::trace::InMemorySpanExporter;
use telemetry::logging::Logger;
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};
use telemetry::tracing::Tracing;
use tokio::sync::watch;

/// Routes nothing.
struct Nothing;

impl Routes for Nothing {
    type Event = ();

    fn decode(&self, _envelope: &Envelope) -> Option<Result<(), String>> {
        None
    }

    async fn handle(&self, _event_id: &str, _event: &()) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn the_defaults_are_application_conf_s_kafka_settings() {
    assert_eq!(
        Resilience::default(),
        Resilience {
            dlq_retry: RetryConfig {
                max_retries: 3,
                base_delay: Duration::from_millis(200),
            },
            dlq_publish_retry: RetryConfig {
                max_retries: 3,
                base_delay: Duration::from_millis(100),
            },
            dlq_publish_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: Duration::from_secs(30),
            },
            restart_base_delay: Duration::from_secs(1),
            restart_max_delay: Duration::from_secs(30),
        }
    );
}

#[tokio::test]
async fn a_consumer_told_to_stop_returns_cleanly() {
    let tracing = Tracing::with_exporter("test", 1.0, InMemorySpanExporter::default());
    let telemetry = ConsumerTelemetry {
        tracer: tracing.tracer(),
        logger: Logger::to_writer("test", Arc::new(Mutex::new(Vec::<u8>::new()))),
        metrics: Metrics::new("test"),
    };
    let settings = ConsumerSettings {
        // Nothing listens: stopping must not need a broker.
        bootstrap: "127.0.0.1:9".to_owned(),
        group_id: "group".to_owned(),
        topic: "topic".to_owned(),
        lag_interval: Duration::from_secs(5),
    };
    let (_stop, stopped) = watch::channel(true);

    let ran = tokio::time::timeout(
        Duration::from_secs(10),
        run_supervised(
            settings,
            &Resilience::default(),
            Nothing,
            telemetry,
            stopped,
        ),
    )
    .await
    .expect("returns, not restarts");

    assert!(ran.is_ok(), "{ran:?}");
}

#[test]
fn its_error_is_nameable_without_depending_on_rdkafka() {
    // An adopter names the error through kafka-client's re-export.
    let error: KafkaError = KafkaError::Canceled;
    assert!(matches!(error, KafkaError::Canceled));
}
