# Spec: shared CI and verify scripts, Linux and macOS (gluon TD-43.2)

## Overview
Both Rust services copy `scripts/ci.sh`'s steps and `scripts/lib/verify.sh`
(gluon TD-43). They move here, once, and run on both Linux (the sandbox,
GitHub Actions) and macOS (the laptop: bash 3.2, BSD tools, no `timeout`).
The services move onto them in their own tracks.

## Functional Requirements
- FR-1 `scripts/lib/portable.sh`: `with_timeout SECONDS COMMAND…` (the
  command's status, 124 when it runs longer; no coreutils `timeout` needed);
  `owner_of PATH` (GNU or BSD `stat`).
- FR-2 `scripts/lib/ci.sh`: `step`, `ci_fmt`, `ci_clippy`, `ci_coverage`
  (the 80% gate, saying why when it fails), `ci_audit [CRATE…]` (SKIP when
  the advisory database is unreachable or cargo-audit missing; FAIL when a
  named lockfile-only crate is built; then `cargo audit --deny warnings`).
  Step names as today (`scripts/verify-rust-ci.sh` in user-service-rs
  matches them).
- FR-3 `scripts/lib/verify.sh`: the services' copy, owner check through
  `owner_of`.
- FR-4 `scripts/ci.sh` on the library, plus a step running
  `scripts/test-lib.sh`.

## Acceptance Criteria
- `scripts/test-lib.sh` green on macOS and on Linux (a `rust:1.99.1-bookworm`
  container).
- `scripts/ci.sh` green; shellcheck clean.

## Out of Scope
- Moving the services onto the library (their own tracks).
