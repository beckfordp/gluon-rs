# Plan — one supervised-consumer helper, one OTEL reader (gluon RPN-11-cleanup)

Spec: [spec.md](./spec.md). Each task red first where it adds behaviour; the
adopting repos' commits are named in their tasks.

## Phase 1: gluon-rs `[checkpoint: 6b3a841]`

- [x] Task 1.1: `telemetry::config` — `TracingConfig`, `tracing_from_env` (S-2) `666c9be`
- [x] Task 1.2: `kafka-client::supervised` — `Resilience`, `ConsumerTelemetry`, `run_supervised` (S-1) `6b38da5`
- [x] Task 1.3: `tech-stack.md` lists `kafka-client` and the two additions (S-5) `6b3a841`

## Phase 2: Adoption and regression `[checkpoint: user-service-rs 16d81e7]`

- [x] Task 2.1: notification-service-rs adopts both (its repo) (notification-service-rs `156e9c5`)
- [x] Task 2.2: user-service-rs and its events-probe adopt both (its repo) (user-service-rs `16d81e7`)
- [x] Task 2.3: Regression — gluon-rs CI, user-service 128/128, notification-service 30/30

## Archive (2026-11-18)

Done (gluon RPN-11-cleanup): `kafka_client::supervised` (`run_supervised`,
`Resilience`, `ConsumerTelemetry`; `KafkaError` re-exported) and
`telemetry::config::tracing_from_env`, adopted by notification-service-rs,
user-service-rs and its events-probe; no copy left (S-1, S-2); S-5 in
tech-stack.md. Regression: gluon-rs CI; user-service 128/128 before and after
(probe 10/10); notification-service 30/30. No track script: the acceptance is
gluon `bin/verify-complete-RPN-11.sh`.
