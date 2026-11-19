# Spec — `user-client`: the user-service lookup, shared (gluon TD-59)

## Overview
notification-service's `UserClient` (ADR 0025: `GET {base}/users/{id}` through purerest's
retry and circuit breaker) was ported for user-service-rs's events-probe
(`events-probe/src/lookup.rs`). notification-service-rs's order-, payment- and
fulfilment-events consumers need it in the session (RPN-12; today
`LookupError::NotPorted`). One copy, in gluon-rs, used by both.

## Functional Requirements
1. Crate `user-client`: `UserClient::new(base_url, timeout, retry, breaker, tracer,
   metrics)` and `UserClient::from_client(base_url, ResilientClient)`;
   `email(user_id) -> Result<Option<String>, LookupError>`: 200 → `Some(email)`,
   400/404 → `None`, anything else → `LookupError::Status`; no response →
   `LookupError::Client`; a 200 body that isn't a user → `LookupError::Body`.
2. The events-probe uses it; its `lookup.rs` and `LookupError` go.

## Acceptance Criteria
- [ ] `user-client` tests for each outcome against a local server; gluon-rs `ci.sh` green.
- [ ] user-service-rs `ci.sh` green; gluon `bin/verify-port user-service` 128/128, probe
      10/10.
- [ ] One copy: no `/users/` request code in events-probe.

## Out of Scope
notification-service-rs's resolver using it (RPN-12, in the session).
