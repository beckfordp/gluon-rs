// SPDX-License-Identifier: MIT

//! `/health`, `/health/ready` and tapir's plain-text errors, as both Scala
//! services answer them (captures `http/health.txt`, `health-head.txt`,
//! `health-ready*.txt`, `unknown-path.txt`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use service_basics::health::{self, Readiness};
use service_basics::text_error::{TEXT_PLAIN, TextError, method_not_allowed, not_found};
use tower::ServiceExt;

/// A service whose dependencies are up or down, as the test says.
#[derive(Clone)]
struct Fake(bool);

impl Readiness for Fake {
    fn ready(&self) -> impl Future<Output = bool> + Send {
        std::future::ready(self.0)
    }
}

fn app(ready: bool) -> Router {
    health::routes()
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .with_state(Fake(ready))
}

/// Status, Content-Type and body.
async fn call(app: Router, method: &str, path: &str) -> (StatusCode, Option<String>, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_owned());
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        content_type,
        String::from_utf8(body.to_vec()).unwrap(),
    )
}

#[tokio::test]
async fn health_is_200_with_no_body_and_no_content_type() {
    assert_eq!(
        call(app(false), "GET", "/health").await,
        (StatusCode::OK, None, String::new())
    );
}

#[tokio::test]
async fn ready_is_200_when_ready_and_503_when_not_with_no_body() {
    assert_eq!(
        call(app(true), "GET", "/health/ready").await,
        (StatusCode::OK, None, String::new())
    );
    assert_eq!(
        call(app(false), "GET", "/health/ready").await,
        (StatusCode::SERVICE_UNAVAILABLE, None, String::new())
    );
}

#[tokio::test]
async fn head_and_other_methods_are_a_plain_text_405() {
    for path in ["/health", "/health/ready"] {
        for method in ["HEAD", "POST", "DELETE"] {
            let (status, content_type, body) = call(app(true), method, path).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
            assert_eq!(content_type.as_deref(), Some(TEXT_PLAIN), "{method} {path}");
            if method != "HEAD" {
                assert_eq!(body, "Method Not Allowed", "{method} {path}");
            }
        }
    }
}

#[tokio::test]
async fn an_unknown_path_is_a_plain_text_404() {
    assert_eq!(
        call(app(true), "GET", "/nope").await,
        (
            StatusCode::NOT_FOUND,
            Some(TEXT_PLAIN.to_owned()),
            "Not found".to_owned()
        )
    );
}

#[tokio::test]
async fn a_text_error_is_its_status_and_message_as_tapir_writes_it() {
    let app = Router::new().route(
        "/bad",
        axum::routing::get(|| async {
            TextError {
                status: StatusCode::BAD_REQUEST,
                message: "Invalid value for: body".to_owned(),
            }
        }),
    );
    assert_eq!(
        call(app, "GET", "/bad").await,
        (
            StatusCode::BAD_REQUEST,
            Some("text/plain; charset=UTF-8".to_owned()),
            "Invalid value for: body".to_owned()
        )
    );
}
