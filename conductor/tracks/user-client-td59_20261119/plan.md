# Plan — `user-client` (gluon TD-59)

Spec: [spec.md](./spec.md).

## Phase 1: The crate `[checkpoint: b098db0]`

- [x] Task 1.1: `user-client` with `UserClient` and `LookupError` `94243a5`
  - [x] Red: found, unknown (404), malformed id (400), 5xx after retries, no answer, a
        body that isn't a user; the request's path
  - [x] Green: moved from events-probe's `lookup.rs`; `ci.sh`
- [x] Task: Conductor - User Manual Verification 'Phase 1: The crate' (Protocol in workflow.md)
      — `scripts/verify-td59.sh 1`: PASS (prompting off)

## Phase 2: Adoption and regression

- [x] Task 2.1: events-probe uses it (user-service-rs track `user-client-td59_20261119`) — `39db09c`
- [x] Task 2.2: gluon `bin/verify-port user-service`; TD-59 ticked; cheat-sheet note for
      RPN-12 (notification's resolver)
- [x] Task: Conductor - User Manual Verification 'Phase 2: Adoption and regression' (Protocol in workflow.md)
      — `scripts/verify-td59.sh 2`: PASS (prompting off)

## Archive (2026-11-20)

TD-59 done: crate `user-client` (`UserClient`, `LookupError`), 100% lines; the events-probe uses it (user-service-rs `39db09c`; 128/128, probe 10/10). notification-service-rs's resolver adopts it in the session (RPN-12; noted for the cheat sheet).
