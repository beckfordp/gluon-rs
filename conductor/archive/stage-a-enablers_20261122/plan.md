# Plan — gluon-rs enablers (gluon all-Rust Stage A P0.3)

Spec: [spec.md](./spec.md).

## Phase 1: HTTP edge and docs

- [x] Task 1.1: CORS allowed headers per policy (tests red first)
- [x] Task 1.2: OpenAPI methods, query parameters, list bodies, nested schemas (tests red first)
- [x] Task 1.3: AsyncAPI built-in page and empty document (tests red first)
- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md): `48d39c4`, scripts/ci.sh green

## Phase 2: Stores

- [x] Task 2.1: service-testing Redis `189f762`
- [x] Task 2.2: `redis-cache` crate, fail-fast when down, reconnect when back, cache-aside `get_or_load` (tests red first) `8f075bc`
- [x] Task 2.3: service-basics outbox writer (tests red first) `a1188e4`
- [x] Task 2.4: both Rust ports' CI green on the new gluon-rs (notification 96.77%, user 96.74% lines); audit ignores `6c74342`
- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md): scripts/ci.sh green, 93.92% lines; bin/check-locks OK

## Archive (2026-11-22)

Reviewed (iterative, 22 files). No High/Medium findings. Low, kept: with Redis
unreachable (not refused), `get_or_load` can spend the timeout twice (get, then set);
refused connections fail at once, and the bound is still the caller's timeout × 2.
The outbox row id is distinct from the envelope's `eventId`, as in payment-service's
`PaymentStore` (`outboxId` vs `eventId`). url 2.5.8 advisory ignored until 2.5.9 is
14 days old (2026-12-04).
