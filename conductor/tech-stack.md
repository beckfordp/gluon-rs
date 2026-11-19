# Tech Stack — gluon-rs

## Environment
As the services: the laptop and the sandbox (gluon ADR 0028; no Docker or
GitHub in the sandbox).

## Language
- Rust 1.99.1, pinned in `rust-toolchain.toml`; edition 2024.
- Workspace lints as the services: `unsafe_code` forbidden, clippy `all` and
  `pedantic`, `unwrap_used`/`expect_used`/`panic` denied outside tests.

## Crates
- `service-basics`: tokio, axum 0.8, sqlx 0.8 (postgres, runtime-tokio),
  thiserror 2. Its `fixture` binary (`src/bin/fixture.rs`) is a minimal
  service for service-testing's tests; consumers never build it.
- `service-testing`: tempfile, ureq 3. A test helper: panics allowed
  crate-wide, as in the services' test support.
- `telemetry`: tracing middleware, metrics, log format, error envelope, the
  docs routes (OpenAPI, Swagger UI, AsyncAPI); the seed of a Rust purerest
  equivalent. Moved from user-service-rs *(2026-11-17, gluon RPN-7)*;
  opentelemetry 0.33 (OTLP/HTTP), schemars 1. `scripts/vendor-swagger-ui.sh`
  refreshes its Swagger UI assets, `scripts/java-uuid-vectors.sh` its JDK
  UUID vectors.
- `kafka-event-consumer`: purekafka's consumer side without a Kafka client —
  envelope routing (`Routes`, `handle_record`), the DLQ guard, the stream
  supervisor, event age and lag, the consume span. The caller's loop owns
  the broker (rdkafka), offsets and the DLQ publisher. Moved from
  user-service-rs's events-probe *(2026-11-17, gluon RPN-8)*. The envelope's
  `timestamp` is decoded as an instant (RFC 3339), so an envelope whose
  timestamp isn't one is poison, as in Scala *(2026-11-18, RPN-11-could-fix,
  TECHNICAL-DEBT S-3)*.
- `kafka-client`: the client side on rdkafka 0.39 (librdkafka, built from
  source): the consume loop (`EventConsumer`, at least once, commit after
  handling, earliest offset for a new group), the dead-letter producer
  (`KafkaPublisher`), the statistics context feeding the lag gauge. Moved from
  the events-probe *(gluon TD-S5)*. `supervised::run_supervised` assembles
  and supervises one consumer from its settings, `Resilience` (purekafka's
  consumer-side settings; `Default` = notification-service's
  `application.conf`) and routes *(2026-11-18, gluon RPN-11-cleanup,
  TECHNICAL-DEBT.md S-1)*. A null-key record is dead-lettered with the key
  `""`, as Scala's consumers do *(2026-11-18, RPN-11-could-fix, S-4)*.
  Typed errors, no `Result<_, String>`: `Routes` and `Publish` declare their own
  `type Error`; dead-lettering fails with `DeadLetterError`, a stream with
  `ConsumeError` (each reads as the message it replaced). An injected `Clock`
  (`SystemClock` in services) times each record's age; the per-record sync
  commit runs off the async worker (`blocking::off_worker`); the consumer's
  metrics are built once; `ConsumerSettings` takes `GroupId` and `Topic`,
  parsed where config is read (`SettingsError`) *(2026-11-19, gluon TD-64)*.
- `telemetry::config::tracing_from_env` reads the OTEL variables for every
  service *(2026-11-18, RPN-11-cleanup, S-2)*.

## Consumers
Path dependencies from `services/<name>-rs`; gluon `bin/k8s-local-up` passes
this repo to `docker build` as a named build context.

## CI
`scripts/ci.sh` runs `scripts/lib/ci.sh`'s steps, which the Rust services' CI
shares: the script library's tests (`scripts/test-lib.sh`), fmt, clippy,
cargo-llvm-cov (at least 80% of lines), cargo audit (skipped when the advisory
database is unreachable). `scripts/lib/` (`ci.sh`, `verify.sh`, `portable.sh`)
runs on Linux and macOS (bash 3.2, BSD tools, no coreutils `timeout`).

A Linux test bed for the laptop: `scripts/on-linux.sh REPO_PATH COMMAND…`
runs any Rust repo's script on Linux in Docker (`scripts/linux-bed/Dockerfile`:
rust:1.99.1-bookworm, PostgreSQL 16, cargo-llvm-cov, cargo-audit) against a
copy of gluon's layout; the host's repos are only read. CI never needs it.
