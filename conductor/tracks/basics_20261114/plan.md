# Plan — service-basics and service-testing (gluon TD-43.1)

`service-basics`' Postgres tests use `service-testing`'s cluster, so that
comes first.

## Phase 1: service-basics
- [x] Task: `service-testing::Postgres` and `free_port` 8e448c0
    - [x] Red: start, query, stop, start_again, pause, resume, pg_dump
    - [x] Green: lifted from the services' `tests/support`
- [ ] Task: `config`
    - [ ] Red: bad ports all reported with today's wording; Postgres defaults
          and overrides; `ConfigError`'s Display
    - [ ] Green
- [ ] Task: `text_error` and `health`
    - [ ] Red: `oneshot` against a fake `Readiness`: 200 empty with no
          Content-Type, ready 200/503, HEAD 405, unknown 404, the text
          headers
    - [ ] Green
- [ ] Task: `postgres`
    - [ ] Red: connect and ping ok; `Timeout` while paused; a wrong address
          fails at connect
    - [ ] Green
- [ ] Task: `environment`, `runtime`, `shutdown` (shutdown tested through the
      fixture binary in Phase 2)
- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: service-testing's Service
- [ ] Task: `Service` and a fixture binary on service-basics
    - [ ] Red: start waits for `/health`; stdout and stderr; SIGTERM exits 0;
          wait_exit; the request helpers
    - [ ] Green
- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)
