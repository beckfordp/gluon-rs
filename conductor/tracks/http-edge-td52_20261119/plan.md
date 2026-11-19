# Plan — the HTTP edge in service-basics (gluon TD-52.3)

Spec: [spec.md](./spec.md).

## Phase 1: service-basics

- [ ] Task 1.1: `cors` module (`CorsPolicy`, `cors`, `Unrouted`, `allows`, `split_names`)
  - [ ] Red: notification-service-rs's CORS tests on a fixture router; the exposed header
  - [ ] Green: moved from notification-service-rs, exposure a policy parameter
- [ ] Task 1.2: `text_error::without_allow`
  - [ ] Red: a 405 has no `Allow`, for a layered router and a fallback service
  - [ ] Green; `ci.sh`
- [ ] Task: Conductor - User Manual Verification 'Phase 1: service-basics' (Protocol in workflow.md)

## Phase 2: Adoption and regression

- [ ] Task 2.1: notification-service-rs adopts (its own track `http-edge-td52_20261119`)
- [ ] Task 2.2: user-service-rs adopts (its own track `http-edge-td52_20261119`)
- [ ] Task 2.3: gluon `bin/verify-port` both; TD-52.3 ticked
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Adoption and regression' (Protocol in workflow.md)
