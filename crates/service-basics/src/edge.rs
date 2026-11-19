// SPDX-License-Identifier: MIT

//! The HTTP edge every Scala service shares (gluon TD-52.3): http4s's CORS
//! outermost ([`crate::cors`]), and tapir's 405 without `Allow`
//! ([`crate::text_error::without_allow`]).

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};

use crate::cors::{CorsPolicy, cors};
use crate::text_error::without_allow;

/// `routes` behind the edge. Wraps the finished router, its own layers and
/// fallbacks included: axum adds `Allow` outside a router's layers, so
/// `Router::layer(without_allow)` alone leaves it on.
pub fn edge(routes: Router, policy: CorsPolicy) -> Router {
    Router::new()
        .fallback_service(routes)
        .layer(from_fn(without_allow))
        .layer(from_fn_with_state(policy, cors))
}
