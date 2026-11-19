# Plan — the HTTP edge in service-basics (gluon TD-52.3)

Spec: [spec.md](./spec.md).

## Phase 1: service-basics

- [x] Task 1.1: `cors` module (`CorsPolicy`, `cors`, `Unrouted`, `allows`, `split_names`) `aea98c1`
  - [x] Red: notification-service-rs's CORS tests on a fixture router; the exposed header
  - [x] Green: moved from notification-service-rs, exposure a policy parameter
- [x] Task 1.2: `text_error::without_allow` `aea98c1`
  - [x] Red: a 405 has no `Allow`, for a layered router and a fallback service
  - [x] Green; `ci.sh`
  - Found: `Router::layer(without_allow)` leaves axum's `Allow` (it's added outside a
    router's layers; user-service-rs's bug). So `edge::edge(routes, policy)` wraps the
    finished router; tests run it bare and with route layers. Mutation-checked.
- [x] Task: Conductor - User Manual Verification 'Phase 1: service-basics' (Protocol in workflow.md)
      — `scripts/verify-td52.sh 1`: PASS (prompting off)

## Phase 2: Adoption and regression

- [ ] Task 2.1: notification-service-rs adopts (its own track `http-edge-td52_20261119`)
- [ ] Task 2.2: user-service-rs adopts (its own track `http-edge-td52_20261119`)
- [ ] Task 2.3: gluon `bin/verify-port` both; TD-52.3 ticked
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Adoption and regression' (Protocol in workflow.md)
