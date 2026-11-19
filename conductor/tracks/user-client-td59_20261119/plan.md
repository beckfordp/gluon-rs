# Plan — `user-client` (gluon TD-59)

Spec: [spec.md](./spec.md).

## Phase 1: The crate

- [x] Task 1.1: `user-client` with `UserClient` and `LookupError` `94243a5`
  - [x] Red: found, unknown (404), malformed id (400), 5xx after retries, no answer, a
        body that isn't a user; the request's path
  - [x] Green: moved from events-probe's `lookup.rs`; `ci.sh`
- [x] Task: Conductor - User Manual Verification 'Phase 1: The crate' (Protocol in workflow.md)
      — `scripts/verify-td59.sh 1`: PASS (prompting off)

## Phase 2: Adoption and regression

- [ ] Task 2.1: events-probe uses it (user-service-rs track `user-client-td59_20261119`)
- [ ] Task 2.2: gluon `bin/verify-port user-service`; TD-59 ticked; cheat-sheet note for
      RPN-12 (notification's resolver)
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Adoption and regression' (Protocol in workflow.md)
