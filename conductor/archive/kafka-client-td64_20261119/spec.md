# Spec — kafka-client review findings (gluon TD-64)

## Overview
The sandbox review of `kafka-client` (2026-11-17, TD-64) found three High and two lesser
issues, carried over from events-probe's `run.rs`. RPN-11 adopted `kafka-client` in
notification-service-rs before they were fixed, so they now reach both services and the
probe. Fix them in gluon-rs before six more ports copy the shape (all-Rust sandbox prep,
P2). Each service adopts the change in its own track.

## Functional Requirements
1. **TD-64.1 — typed errors.** No `Result<_, String>` left in `kafka-client` or
   `kafka-event-consumer`:
   - `kafka-client`: a `thiserror` `ConsumeError` (subscribe, all brokers down, commit,
     dead-letter failed) from `EventConsumer::consume`.
   - `kafka-event-consumer`: `DeadLetterError` and `HandleError` for `dead_letter`,
     `handle_record`, `envelope`.
   - The `Routes`, `Publish` and `decode` traits declare
     `type Error: std::error::Error + Send + Sync + 'static`, so each service keeps its
     own error enum.
   - `supervise` is generic over `E: Display`.
   - `run_supervised` still returns `KafkaError` at the edge, so callers are unchanged
     apart from the trait's associated types.
2. **TD-64.2 — injected clock.** `stream.rs` no longer reads `SystemTime::now()`. A
   `Clock` trait (`fn now(&self) -> SystemTime`) with a `SystemClock` default sits on
   `EventConsumer` and is wired in by `run_supervised` / `main`. A test fixes the clock
   and asserts the age recorded.
3. **TD-64.3 — non-blocking commit.** The per-record
   `commit_message(…, CommitMode::Sync)` runs inside `tokio::task::block_in_place`. It
   stays at-least-once: commit after processing, same order.
4. **TD-64.4 — metrics built once.** `ConsumerMetrics` is built once per `consume` and
   shared with `subscribe`'s `LagContext`.
5. **TD-64.5 — newtypes.** `ConsumerSettings`' `group_id` and `topic` become `GroupId`
   and `Topic`, parsed where config is read: non-empty, Kafka's legal characters,
   length ≤ 249. An invalid value is a config error at startup.

## Non-Functional Requirements
- Behaviour on the wire is unchanged: the same records, headers, DLQ records, metrics
  and logs.
- gluon-rs `scripts/ci.sh` passes: fmt, clippy pedantic, tests, coverage ≥ 80%
  (`stream.rs` keeps its broker-only exclusion).
- `rust.md` checklist: no new `unwrap`/`expect` outside tests, and the errors carry a
  source chain.

## Acceptance Criteria
- [ ] `grep -rn 'Result<.*String>' crates/kafka-client crates/kafka-event-consumer`
      finds nothing.
- [ ] The clock test and the newtype parse tests pass; a test proves the commit path
      runs in `block_in_place`, or a code review note shows it does if that can't be
      tested.
- [ ] gluon-rs `ci.sh` passes.
- [ ] Adoption tracks (out of this track, but gate its close): user-service-rs `ci.sh`
      passes with probe 10/10 and 128/128; notification-service-rs `ci.sh` passes with
      38/38.
- [ ] gluon `TECHNICAL_DEBT.md`: TD-64.1–64.5 ticked, and TD-64 archived.

## Out of Scope
- Retry, breaker or DLQ semantics.
- Producer-side (`KafkaPublisher`) changes beyond its `Publish` error type.
- Async commit or batching.
- Changes to the services beyond adopting the new types (that's their own tracks).
