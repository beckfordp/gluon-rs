# Plan — the Kafka client side moves to gluon-rs (gluon TECHNICAL_DEBT.md TD-S5)

The consume loop, the dead-letter publisher and the librdkafka statistics context
(the lag gauge) live in user-service-rs's events-probe (`src/run.rs`), while
`kafka-event-consumer` deliberately has no Kafka client. notification-service-rs's
consumers need exactly that code. It moves here, as a crate of its own, so every
port's consumers share one loop; events-probe becomes its first user.
Agreed with the owner 2026-11-17. Tasks marked **(user-service-rs)** are made
there, one commit each; the rest are this repo's.

## Decisions (2026-11-17; the first three are proposals for the owner's review)
- **A crate of its own, `kafka-client`**, not a feature of `kafka-event-consumer`:
  that crate stays client-free (its tests build no librdkafka). It holds
  `KafkaPublisher` (dead letters, `Publish`), `LagContext` (statistics → lag),
  `ConsumerSettings` and `EventConsumer<R: Routes, P: Publish>` (the loop: age,
  span, `handle_record` under the DLQ guard, commit).
- **The loop isn't measured for coverage here** (`ci_coverage`'s ignore regex), as
  in user-service-rs today: it needs a broker, which CI hasn't, and the contract
  harness runs it (the probe group; notification-service's consumer groups). The
  rest is unit-tested.
- **The lock.** rdkafka now enters gluon-rs's `Cargo.lock`. The sandbox's crates.io
  has libz-sys 1.1.31 at most; the laptop uses 1.1.33 (user-service-rs's committed
  lock). The committed lock carries 1.1.33 and the versions user-service-rs's
  committed lock has; the sandbox builds with 1.1.31, uncommitted
  (TECHNICAL_DEBT.md TD-S10, extended to gluon-rs).
- notification-service-rs depends on `kafka-client` when its first consumer is built
  (RPN-10's green phase), not in this track.
- Review before archive (owner, 2026-11-17).

## Phase 1: `kafka-client` in gluon-rs (TDD)
- [x] 1.1 The crate and its workspace entry; `ConsumerSettings` → librdkafka config
      (unit-tested: group, no auto-commit, earliest, statistics interval)
- [x] 1.2 `LagContext`: statistics become the lag gauge (the probe's test, moved)
- [x] 1.3 `KafkaPublisher` (creation tested; delivery needs a broker); header
      extraction, a pure `text_headers` (unit-tested)
- [x] 1.4 `EventConsumer`: `process` (span, `handle_record`, DLQ guard; unit-tested
      with fakes) in `consumer.rs`; `consume` and `subscribe` (age, commit; fails on
      all-brokers-down or a failed commit) in `stream.rs`, the only file not measured
- [x] 1.5 The lock as decided (only libz-sys differs from the laptop's versions: the
      commit carries 1.1.33, the sandbox builds 1.1.31); gluon-rs `scripts/ci.sh` green
      (lines 93.04%)

## Phase 2: events-probe uses it
- [x] 2.1 **(user-service-rs)** events-probe builds an `EventConsumer` from its config;
      `src/run.rs` deleted; `kafka-client` by path; the moved test gone from
      `tests/probe.rs`; `scripts/ci.sh`'s ignore regex is `main.rs` only; the probe's
      `ProbeConfig::consumer_settings` (tested) keeps `main`'s `run` under clippy's limit
- [x] 2.2 **(user-service-rs)** `scripts/ci.sh` green (lines 93.28%, was 93.23%)

## Phase 3: Full regression
- [x] 3.1 CI of gluon-rs (lines 93.04%), user-service-rs (93.28%), notification-service-rs
      (97.03%, 43 tests) and tests/contract (420 passed, 95.52%)
- [x] 3.2 `bin/verify-port user-service --only build contract`: 128/128 (probe 10/10:
      the moved loop against a real broker), C1, C8, C10 MET
- [x] 3.3 notification-service: `verify-check-group.sh` REST `--expect-green` (24/24),
      user-events `--expect-red "not yet implemented"` (6)
- [x] 3.4 **(gluon)** TECHNICAL_DEBT.md: TD-S5 resolved; TD-S10 covers gluon-rs's lock

## Phase 4: Review, then archive
- [x] 4.1 Review of the track's changes (here and in user-service-rs) against the
      plan, the product guidelines and the style guides; report to the owner
      (H1–H3 and two Lows, all carried over from `run.rs`: gluon TECHNICAL_DEBT.md
      TD-S19 to TD-S23, H1–H3 first in the notification-service consumer work)
- [x] 4.2 Archive once the owner approves (approved 2026-11-17)
