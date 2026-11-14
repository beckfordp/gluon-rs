// SPDX-License-Identifier: MIT

//! `/health` and `/health/ready`, as both Scala services answer them
//! (captures `http/health.txt`, `health-head.txt`, `health-ready*.txt`).

use std::future::Future;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;

use crate::text_error::method_not_allowed;

/// Whether the service's dependencies answer, for `/health/ready`. Each
/// service decides what that means (and whether it is measured).
pub trait Readiness {
    /// True when the service can serve requests.
    fn ready(&self) -> impl Future<Output = bool> + Send;
}

/// The two routes, for the service to merge into its router before
/// `with_state`. HEAD is a 405, as tapir answers it.
pub fn routes<S>() -> Router<S>
where
    S: Readiness + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health).head(method_not_allowed))
        .route("/health/ready", get(ready::<S>).head(method_not_allowed))
}

/// `GET /health`: 200 with an empty body while the process is up; checks no
/// dependencies (capture `http/health.txt`: no Content-Type).
async fn health() -> StatusCode {
    StatusCode::OK
}

/// `GET /health/ready`: 200 when ready, 503 when not; empty body either way.
async fn ready<S: Readiness>(State(state): State<S>) -> StatusCode {
    if state.ready().await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
