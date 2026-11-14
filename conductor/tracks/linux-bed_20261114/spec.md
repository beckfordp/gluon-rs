# Spec: a Linux test bed for the Rust repos' scripts (gluon TD-43.2)

## Overview
The Rust repos' scripts run on macOS (the laptop) and Linux (the sandbox,
GitHub Actions). A one-off Docker run caught a race macOS hid (with_timeout
143, not 124). Make that the test bed: any script, any Rust repo, on Linux,
from the laptop. A laptop tool: the sandbox has no Docker, and CI never needs
it.

## Functional Requirements
- FR-1 `scripts/linux-bed/Dockerfile`: `rust:1.99.1-bookworm` with the pinned
  toolchain's components, PostgreSQL 16 binaries on the PATH (as the GitHub
  workflow), cargo-llvm-cov 0.9.1, cargo-audit, rsync, shellcheck; a non-root
  `tester` (PostgreSQL refuses root).
- FR-2 `scripts/on-linux.sh REPO_PATH COMMAND…`: builds the image when its
  Dockerfile changed; copies gluon's layout (`lib/gluon-rs`, `registry`,
  REPO_PATH; no `target/`, `.git`) from a read-only mount into a volume owned
  by `tester`, keeping the copies' `target/` between runs; runs COMMAND there
  as `tester`; passes its exit status through.

## Acceptance Criteria
- `scripts/verify-linux-bed.sh`: `test-lib.sh` PASS on the bed, a status
  passed through, the host's repos untouched.
- `on-linux.sh lib/gluon-rs scripts/ci.sh` green.
- shellcheck clean.

## Out of Scope
- Running the bed from CI.
