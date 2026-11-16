# Initial Concept

Shared Rust crates for gluon's Rust ports (TD-43): what every port needs,
written once, so a new port (cart next) doesn't start as a copy.

# Product Guide — gluon-rs

## Context
The Rust counterpart of `lib/purerest` (gluon ADR 0005). The vision lives in
gluon: ADR 0028 and 0029, `TECHNICAL_DEBT.md` TD-43, and PLAN.md's Rust-port
sections.

## Purpose
- `service-basics`: environment, runtime, shutdown, config (ports, Postgres,
  `ConfigError`), the Postgres pool and bounded ping, tapir's plain-text
  errors, `/health` and `/health/ready`, and values as the JVM prints them
  (`jvm::java_instant`, both ports' JSON; gluon RPN-5).
- `service-testing`: a throwaway PostgreSQL cluster from the local binaries;
  the service binary under test.
- Later: `telemetry` (RPN-7), the Kafka consumer pieces (RPN-8).

## Users
user-service-rs and notification-service-rs (path dependencies); later ports.

## Goals
No copy of a shared piece in any port; each port keeps only its own
contracts. Behaviour here is what the Scala services do on the wire: a change
here is a change to every port, checked by both services' tests and the
contract harness.
