# Spec — gluon-rs enablers for the all-Rust ports (gluon all-Rust Stage A P0.3)

## Overview
cart, catalog, inventory, payment, order and fulfilment are generated from
tools/rust-service-template; what they share belongs here first, with tests
(gluon `plans/private/retros/all-rust.md`, Prompt A, Phase 0.3).

## Functional requirements
1. service-basics CORS: a policy names its allowed request headers (inventory,
   fulfilment, payment, order allow `Authorization` beside `Content-Type`, as each
   `Cors.scala`); default `Content-Type`, as today.
2. telemetry OpenAPI: DELETE, PUT, PATCH operations; query parameters; a list body
   (`"Product[]"` → array of the component); a component's nested types hoisted into
   `components.schemas` with their `$ref`s rewritten.
3. telemetry AsyncAPI: a page built into the binary (no npm, no files on disk) served at
   `/docs/asyncapi/`, listing every channel, its direction, messages and payload
   schema; a document with no channels ("publishes and consumes nothing").
4. New crate `redis-cache`: get / set with TTL / delete, each bounded by a short
   timeout; Redis down → an error at once, never a multi-second reconnect per command
   (the sandbox's 19 s per request); reconnects on the next call once Redis is back.
5. service-basics outbox: `Outbox::write` inserts the shared envelope into the
   service's `outbox` table inside the caller's transaction, as the Scala services'
   `OutboxWriter` and Debezium's `EventRouter` expect.
6. service-testing: a throwaway `redis-server` on a free port.

## Acceptance criteria
- Each requirement has tests written red first; `scripts/ci.sh` green (fmt, clippy,
  tests, coverage gate, audit).
- notification-service-rs and user-service-rs still build and pass their CI on the new
  gluon-rs (nothing they use changes behaviour).

## Out of scope
The template (its own change, after this track); harness stubs (tests/contract).
