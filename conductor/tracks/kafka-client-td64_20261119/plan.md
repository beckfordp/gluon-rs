# Plan — kafka-client review findings (gluon TD-64)

Spec: [spec.md](./spec.md). Each task is TDD: write the failing test (red), confirm it
fails, implement (green), then refactor and run `scripts/ci.sh`. The adoption commits
land in the services' own tracks; this plan names them.

## Phase 1: Typed errors (TD-64.1, High)

- [ ] Task 1.1: `kafka-event-consumer` errors
  - [ ] Red: tests that `dead_letter` and `handle_record` return `DeadLetterError` /
        `HandleError` variants with a source chain (handler failure, decode failure,
        DLQ publish failed)
  - [ ] Green: the `thiserror` enums; `Routes`, `Publish` and `decode` gain
        `type Error`; `supervise<E: Display>`
  - [ ] Refactor; `ci.sh`
- [ ] Task 1.2: `kafka-client` `ConsumeError`
  - [ ] Red: test that `consume`'s commit and dead-letter failures map to their variants
        (a fake consumer / commit seam where the broker isn't needed)
  - [ ] Green: `ConsumeError` (subscribe, all brokers down, commit, dead-letter);
        `KafkaPublisher`'s `Publish::Error`; `run_supervised` maps to `KafkaError` at
        the edge
  - [ ] Refactor; `ci.sh`; the `grep` for `Result<.*String>` finds nothing
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Typed errors' (Protocol in workflow.md)

## Phase 2: Clock and commit (TD-64.2, TD-64.3, High)

- [ ] Task 2.1: injected `Clock`
  - [ ] Red: a fixed clock, and a record with a known timestamp, gives the expected age
  - [ ] Green: `Clock` trait + `SystemClock`; a field on `EventConsumer`;
        `run_supervised` wires `SystemClock`; `now_ms` gone
  - [ ] Refactor; `ci.sh`
- [ ] Task 2.2: commit in `block_in_place`
  - [ ] Red: where testable, a multi-thread runtime test shows the commit runs off the
        async worker; otherwise a review note in the plan
  - [ ] Green: `tokio::task::block_in_place(|| consumer.commit_message(…,
        CommitMode::Sync))`; order and at-least-once unchanged
  - [ ] Refactor; `ci.sh`
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Clock and commit' (Protocol in workflow.md)

## Phase 3: Metrics once and newtypes (TD-64.4, TD-64.5)

- [ ] Task 3.1: `ConsumerMetrics` built once
  - [ ] Red: `subscribe` takes the built `ConsumerMetrics` (compile-level), or a test
        counts the families registered once
  - [ ] Green: built in `consume`, cloned into `LagContext`
- [ ] Task 3.2: `GroupId` and `Topic` newtypes
  - [ ] Red: parse tests (empty, illegal characters, length > 249, valid)
  - [ ] Green: newtypes with `FromStr` / `TryFrom<String>` and `Display`;
        `ConsumerSettings` uses them; a config error on an invalid value
  - [ ] Refactor; `ci.sh`; `tech-stack.md` notes the newtypes and the error types
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Metrics once and newtypes' (Protocol in workflow.md)

## Phase 4: Adoption and regression

- [ ] Task 4.1: notification-service-rs adopts (its own track
      `kafka-client-td64_20261119`): handler error enums as `type Error`,
      `GroupId`/`Topic` from config; `ci.sh`; 38/38
- [ ] Task 4.2: user-service-rs and its events-probe adopt (its own track
      `kafka-client-td64_20261119`): `ci.sh`; probe 10/10; 128/128
- [ ] Task 4.3: Regression — gluon-rs `ci.sh`; `bin/verify-port user-service` and
      `notification-service` pass (cluster not run on macOS); gluon
      `TECHNICAL_DEBT.md` TD-64 ticked and archived
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Adoption and regression' (Protocol in workflow.md)
