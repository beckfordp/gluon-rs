# Product Guidelines — gluon-rs

## Parity first
A crate here carries behaviour both services already have on the wire, cited
to the Scala source or a capture; nothing moves here until two ports need it.
A service-specific value (pool size, defaults, variable names) is a
parameter, cited where the service passes it.

## Code conventions
[`code_styleguides/rust.md`](./code_styleguides/rust.md) (copied from
user-service-rs) and [`general.md`](./code_styleguides/general.md).

## API design
Small, explicit APIs: plain functions and types, no framework of our own;
services wire them in their own `main` and router.

## Testing
`cargo fmt --check`, `cargo clippy -- -D warnings`, tests covering at least
80% of lines (`scripts/ci.sh`). Postgres tests run on a throwaway PostgreSQL
16 cluster, no Docker. A change here is done when both services' `ci.sh` are
green on it.

## Security
No secrets committed; configuration from env vars only.
