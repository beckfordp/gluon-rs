# Spec: service-basics and service-testing (gluon TD-43.1)

## Overview
The pieces both Rust services copy (gluon TD-43), as two crates. The services
move onto them in their own tracks.

## Functional Requirements
- FR-1 `service-basics`:
  - `environment::variables()`: the process environment as a map of the
    UTF-8 variables
  - `runtime::multi_thread()`
  - `shutdown::signal(service)`: completes on SIGTERM or SIGINT; if SIGTERM
    can't be watched, says so and waits for SIGINT
  - `config`: `ConfigError` (`invalid configuration:\n- …`), `PostgresConfig`,
    `port(env, name, default, problems)` with today's wording,
    `postgres(env, defaults)`
  - `postgres`: `connect(config, pool_size)` (3 s acquire timeout, one
    connection opened at startup); `ping(pool, within) -> Result<(),
    PingError>` (`Timeout` or `Sql`)
  - `text_error`: `TEXT_PLAIN`, `TextError`, `method_not_allowed()`,
    `not_found()`
  - `health`: a `Readiness` trait; `routes::<S>()` for `/health` (200, empty,
    no Content-Type) and `/health/ready` (200 or 503), HEAD 405
- FR-2 `service-testing`: `free_port`; `Postgres` (start, stop, start_again,
  pause, resume, env, query, pg_dump); `Service::spawn(binary, port_var,
  env)` / `start`, and the request, output and stop helpers, as both
  services' `tests/support` do today.

## Acceptance Criteria
- `scripts/ci.sh` green (at least 80% of lines).
- Each behaviour unit- or Postgres-tested.

## Out of Scope
- Moving the services onto the crates (their own tracks).
- `telemetry` (RPN-7); `verify.sh`/`ci.sh` sharing (TD-43.2).
