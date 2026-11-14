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
