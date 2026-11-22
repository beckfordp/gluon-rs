# Plan — gluon-rs enablers (gluon all-Rust Stage A P0.3)

Spec: [spec.md](./spec.md).

## Phase 1: HTTP edge and docs

- [ ] Task 1.1: CORS allowed headers per policy (tests red first)
- [ ] Task 1.2: OpenAPI methods, query parameters, list bodies, nested schemas (tests red first)
- [ ] Task 1.3: AsyncAPI built-in page and empty document (tests red first)
- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Stores

- [ ] Task 2.1: service-testing Redis
- [ ] Task 2.2: `redis-cache` crate, fail-fast when down, reconnect when back (tests red first)
- [ ] Task 2.3: service-basics outbox writer (tests red first)
- [ ] Task 2.4: both Rust ports' CI green on the new gluon-rs
- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)
