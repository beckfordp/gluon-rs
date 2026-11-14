# Tech Stack — gluon-rs

## Environment
As the services: the laptop and the sandbox (gluon ADR 0028; no Docker or
GitHub in the sandbox).

## Language
- Rust 1.99.1, pinned in `rust-toolchain.toml`; edition 2024.
- Workspace lints as the services: `unsafe_code` forbidden, clippy `all` and
  `pedantic`, `unwrap_used`/`expect_used`/`panic` denied outside tests.

## Crates
- `service-basics`: tokio, axum 0.8, sqlx 0.8 (postgres, runtime-tokio).
- `service-testing`: tempfile, ureq 3. A test helper: panics allowed
  crate-wide, as in the services' test support.

## Consumers
Path dependencies from `services/<name>-rs`; gluon `bin/k8s-local-up` passes
this repo to `docker build` as a named build context.

## CI
`scripts/ci.sh`: fmt, clippy, cargo-llvm-cov (at least 80% of lines), cargo
audit (skipped when the advisory database is unreachable).
