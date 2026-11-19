// SPDX-License-Identifier: MIT

//! Framework errors as tapir writes them: plain text (capture
//! `http/unknown-path.txt`, `http/*-wrong-method.txt`).

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Exactly what the Scala services send for text errors (not axum's `utf-8`).
pub const TEXT_PLAIN: &str = "text/plain; charset=UTF-8";

/// A framework error: its status, and the whole body as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextError {
    /// The status.
    pub status: StatusCode,
    /// The whole body.
    pub message: String,
}

impl IntoResponse for TextError {
    fn into_response(self) -> Response {
        (
            self.status,
            [(header::CONTENT_TYPE, TEXT_PLAIN)],
            self.message,
        )
            .into_response()
    }
}

/// 405, as tapir answers a known path with another method. HEAD included:
/// tapir doesn't answer HEAD for a GET endpoint (capture `http/health-head.txt`).
pub async fn method_not_allowed() -> TextError {
    TextError {
        status: StatusCode::METHOD_NOT_ALLOWED,
        message: "Method Not Allowed".to_owned(),
    }
}

/// 404 for any unknown path (capture `http/unknown-path.txt`).
// An axum handler (the services' `fallback`), so `async` though it awaits nothing.
#[allow(clippy::unused_async)]
pub async fn not_found() -> TextError {
    TextError {
        status: StatusCode::NOT_FOUND,
        message: "Not found".to_owned(),
    }
}

/// Drops `Allow`: tapir's 405 has none (captures `http/emails-post.txt`,
/// `health-head.txt`), axum's method router adds one. Wrap the whole router
/// (gluon TD-52.3).
pub async fn without_allow(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().remove(header::ALLOW);
    response
}
