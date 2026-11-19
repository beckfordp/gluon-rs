# Spec — the HTTP edge in service-basics (gluon TD-52.3)

## Overview
Every Scala service wraps its routes in the same http4s `CORS.policy` (`Cors.scala`: any
origin; GET, POST, PATCH, PUT, DELETE; `Content-Type`; no credentials; user-service also
exposes `X-Total-Count`) and answers a wrong method with tapir's 405, which has no
`Allow`. notification-service-rs has a faithful copy; user-service-rs a looser one (Allow-*
on a refused preflight, CORS on the 404, `Allow` on its 405s). gshop calls every service
from the browser, and six more ports are coming: one copy, in `service-basics`.

## Functional Requirements
1. `service_basics::cors`: `CorsPolicy` (default: nothing exposed; `exposing(..)`), the
   `cors` middleware (notification-service-rs's behaviour, plus
   `Access-Control-Expose-Headers` on a cross-origin routed response when exposing), and
   the `Unrouted` marker (a response part) the 404 fallback sets so CORS leaves it alone.
2. `service_basics::text_error::without_allow`: middleware dropping axum's `Allow`.
3. Both ports use them; their `cors.rs` and `Allow` code go.

## Acceptance Criteria
- [ ] service-basics tests: notification-service-rs's CORS cases, the exposed header, and
      no `Allow` on a 405 whether the router is layered or a fallback service.
- [ ] No CORS or `Allow` code in either port; `ci.sh` green in all three repos;
      `bin/verify-port` user-service 128/128 and notification-service 38/38.

## Out of Scope
TD-52.1 (harness 405 checks for `Allow`) and TD-52.2 (a user-service refused-preflight
capture): laptop capture and tests/contract work, after the export.
