# Spec — one supervised-consumer helper, one OTEL reader

Track `shared-consumer-otel_20261118`, gluon **RPN-11-cleanup** (with
notification-service-rs's `rust-idioms-cleanup_20261118`). Agreed with the owner
2026-11-18: gluon `TECHNICAL-DEBT.md` item 8 (S-1, S-2) and S-5.

## Problem

- S-1: every consumer is assembled by hand — KafkaPublisher → DeadLetter (dlq-retry,
  dlq-publish-resilience) → EventConsumer → `supervise` — in the events-probe's
  `main.rs` and notification-service-rs's `consumers.rs`; three more consumers
  would copy it again.
- S-2: `OTEL_EXPORTER_OTLP_ENDPOINT` / `OTEL_TRACES_SAMPLER_ARG` are parsed in
  user-service-rs, the probe and notification-service-rs, three copies.
- S-5: `tech-stack.md` doesn't list `kafka-client`.

## Change

- `kafka-client::supervised`: `Resilience` (purekafka's consumer-side settings;
  `Default` = notification-service's `application.conf`: dlq-retry 3 × 200 ms,
  publish 3 × 100 ms, breaker 5 / 30 s, restarts 1 s doubling to 30 s),
  `ConsumerTelemetry` (tracer, logger, metrics) and `run_supervised(settings,
  &resilience, routes, telemetry, stop)`: one consumer, supervised, until `stop`;
  an error only when the DLQ producer can't be built (`KafkaError`).
- `telemetry::config`: `TracingConfig` and `tracing_from_env(env, &mut problems)`,
  user-service-rs's reader moved (same messages), in the shape of
  service-basics' `port` / `postgres`.
- Adopted: notification-service-rs (`consumers.rs`, `config.rs`), user-service-rs
  (`config.rs`; the probe's `main.rs` and `config.rs`). Their copies go.

## Acceptance

gluon `bin/verify-complete-RPN-11.sh` criteria 9–12, 15: the helper and
reader here, no copy left, gluon-rs CI, user-service 128/128 (its probe group
included) and notification-service 30/30.
