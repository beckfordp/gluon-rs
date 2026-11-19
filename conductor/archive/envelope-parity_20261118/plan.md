# Plan — envelope parity on invalid input (gluon RPN-11-could-fix)

Spec: [spec.md](./spec.md). Each task red first.

## Phase 1: Parity `[checkpoint: d8de3d8]`

- [x] Task 1.1: The envelope's timestamp is an instant (S-3) `76da0a4`
- [x] Task 1.2: A null key is dead-lettered as "" (S-4) `d8de3d8`

## Archive (2026-11-18)

Done (gluon RPN-11-could-fix, TECHNICAL-DEBT S-3, S-4): the envelope's timestamp
is an instant; a null key is dead-lettered as "". Both services' locks refreshed
(notification-service-rs `ad22ac2`, user-service-rs `d9c84ad`). Regression:
gluon `bin/verify-complete-RPN-11.sh` (the cleanup's and RPN-11's
acceptance included: user-service 128/128 with its probe group,
notification-service 30/30).
