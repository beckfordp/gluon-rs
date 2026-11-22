// SPDX-License-Identifier: MIT

//! The HTTP edge every Scala service shares (gluon TD-52.3): http4s 0.23
//! `CORS.policy` with `Cors.scala`'s settings, outermost, and tapir's 405
//! without `Allow`. Captures `http/cors-preflight.txt`,
//! `cors-preflight-bad-header.txt`, `cors-simple.txt` (notification-service),
//! `emails-post.txt`, `health-head.txt`; user-service's `X-Total-Count`
//! exposure (its `cors-simple.txt`). Each case runs over a bare router and
//! one with its own route layers (user-service-rs's tracing and metrics).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{Next, from_fn};
use axum::response::Response;
use axum::routing::{any, get};
use service_basics::cors::{CorsPolicy, Unrouted, split_names};
use service_basics::edge::edge;
use service_basics::text_error::{self, TextError, method_not_allowed};
use tower::ServiceExt;

const TEXT: &str = "text/plain; charset=UTF-8";
const ORIGIN: (&str, &str) = ("origin", "http://localhost:5173");
const VARY: &str = "Access-Control-Request-Method, Access-Control-Request-Headers";

/// 404 for an unknown path, marked so CORS leaves it alone.
async fn not_found() -> (Unrouted, TextError) {
    (Unrouted, text_error::not_found().await)
}

/// `/things` answers GET and tapir's 405 for the rest (as `/emails`);
/// `/plain` only GET (HEAD refused, as the ports do), so a wrong method is
/// axum's 405, `Allow` and all.
fn routes() -> Router {
    let things = any(method_not_allowed)
        .get(|| async { "[]" })
        .head(method_not_allowed);
    Router::new()
        .route("/things", things)
        .route("/plain", get(|| async { "plain" }).head(method_not_allowed))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
}

/// A route layer that passes everything through, as tracing and metrics do.
async fn passing(request: Request, next: Next) -> Response {
    next.run(request).await
}

/// Both routers behind the edge, named for the failure messages.
fn apps(policy: &CorsPolicy) -> [(&'static str, Router); 2] {
    [
        ("bare", edge(routes(), policy.clone())),
        (
            "layered",
            edge(routes().layer(from_fn(passing)), policy.clone()),
        ),
    ]
}

struct Answer {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: String,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn names(&self) -> Vec<&str> {
        self.headers.iter().map(|(key, _)| key.as_str()).collect()
    }
}

async fn send(app: Router, method: &str, uri: &str, headers: &[(&str, &str)]) -> Answer {
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str().to_owned(), value.to_str().unwrap().to_owned()))
        .collect();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    Answer {
        status,
        headers,
        body: String::from_utf8(body.to_vec()).unwrap(),
    }
}

/// Each wiring's answer to the same request.
async fn each(
    policy: &CorsPolicy,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
) -> Vec<(&'static str, Answer)> {
    let mut answers = Vec::new();
    for (wiring, app) in apps(policy) {
        answers.push((wiring, send(app, method, uri, headers).await));
    }
    answers
}

async fn preflight(
    path: &str,
    method: &str,
    request_headers: Option<&str>,
) -> Vec<(&'static str, Answer)> {
    let mut headers = vec![ORIGIN, ("access-control-request-method", method)];
    if let Some(names) = request_headers {
        headers.push(("access-control-request-headers", names));
    }
    each(&CorsPolicy::default(), "OPTIONS", path, &headers).await
}

fn assert_allowed(answers: &[(&str, Answer)], what: &str) {
    for (wiring, answer) in answers {
        let what = format!("{wiring}: {what}");
        assert_eq!(answer.status, StatusCode::OK, "{what}");
        assert_eq!(answer.body, "", "{what}");
        assert_eq!(
            answer.header("access-control-allow-origin"),
            Some("*"),
            "{what}"
        );
        assert_eq!(
            answer.header("access-control-allow-methods"),
            Some("PATCH, PUT, GET, POST, DELETE"),
            "{what}"
        );
        assert_eq!(
            answer.header("access-control-allow-headers"),
            Some("Content-Type"),
            "{what}"
        );
        assert_eq!(answer.header("vary"), Some(VARY), "{what}");
        for absent in [
            "access-control-expose-headers",
            "access-control-allow-credentials",
            "access-control-max-age",
        ] {
            assert_eq!(answer.header(absent), None, "{what}: {absent}");
        }
    }
}

fn assert_refused(answers: &[(&str, Answer)], what: &str) {
    for (wiring, answer) in answers {
        let what = format!("{wiring}: {what}");
        assert_eq!(answer.status, StatusCode::OK, "{what}");
        assert_eq!(answer.body, "", "{what}");
        assert_eq!(answer.header("vary"), Some(VARY), "{what}");
        assert!(
            !answer
                .names()
                .iter()
                .any(|name| name.starts_with("access-control-")),
            "{what}: {:?}",
            answer.headers
        );
    }
}

#[tokio::test]
async fn a_preflight_the_policy_allows_gets_every_allow_header() {
    // capture http/cors-preflight.txt
    assert_allowed(
        &preflight("/things", "GET", Some("content-type")).await,
        "as captured",
    );
    for (method, names) in [
        ("DELETE", None),
        ("PATCH", Some("Content-Type")),
        ("POST", Some("CONTENT-TYPE , content-type")),
        ("PUT", Some("content-type,")),
    ] {
        assert_allowed(
            &preflight("/things", method, names).await,
            &format!("{method} {names:?}"),
        );
    }
}

#[tokio::test]
async fn a_preflight_on_any_path_is_answered_by_the_policy() {
    for path in ["/no-such-path", "/plain", "/things"] {
        assert_allowed(&preflight(path, "GET", None).await, path);
    }
}

#[tokio::test]
async fn a_preflight_the_policy_refuses_gets_only_vary() {
    // capture http/cors-preflight-bad-header.txt
    assert_refused(
        &preflight("/things", "GET", Some("authorization")).await,
        "authorization",
    );
    for (method, names) in [
        ("GET", Some("content-type, authorization")),
        ("GET", Some("")),
        ("HEAD", None),
        ("OPTIONS", Some("content-type")),
        ("get", None),
    ] {
        assert_refused(
            &preflight("/things", method, names).await,
            &format!("{method} {names:?}"),
        );
    }
}

#[tokio::test]
async fn a_cross_origin_request_gets_only_allow_origin() {
    // capture http/cors-simple.txt (notification-service)
    for (wiring, answer) in each(&CorsPolicy::default(), "GET", "/things", &[ORIGIN]).await {
        assert_eq!(answer.status, StatusCode::OK, "{wiring}");
        assert_eq!(answer.body, "[]", "{wiring}");
        let cors: Vec<&str> = answer
            .names()
            .into_iter()
            .filter(|name| name.starts_with("access-control-") || *name == "vary")
            .collect();
        assert_eq!(cors, vec!["access-control-allow-origin"], "{wiring}");
    }
    // Errors from a route too.
    for (wiring, post) in each(&CorsPolicy::default(), "POST", "/things", &[ORIGIN]).await {
        assert_eq!(post.status, StatusCode::METHOD_NOT_ALLOWED, "{wiring}");
        assert_eq!(
            post.header("access-control-allow-origin"),
            Some("*"),
            "{wiring}"
        );
    }
}

#[tokio::test]
async fn an_exposing_policy_names_its_headers_on_a_cross_origin_answer() {
    // user-service: Cors.scala's withExposeHeadersIn(X-Total-Count), capture
    // http/cors-simple.txt
    let policy = CorsPolicy::exposing("X-Total-Count");
    for (wiring, answer) in each(&policy, "GET", "/things", &[ORIGIN]).await {
        assert_eq!(
            answer.header("access-control-expose-headers"),
            Some("X-Total-Count"),
            "{wiring}"
        );
        assert_eq!(
            answer.header("access-control-allow-origin"),
            Some("*"),
            "{wiring}"
        );
    }
    for (wiring, answer) in each(&policy, "GET", "/things", &[]).await {
        assert_eq!(
            answer.header("access-control-expose-headers"),
            None,
            "{wiring}: no Origin, no CORS"
        );
    }
    let mut asked = vec![ORIGIN, ("access-control-request-method", "GET")];
    asked.push(("access-control-request-headers", "content-type"));
    for (wiring, answer) in each(&policy, "OPTIONS", "/things", &asked).await {
        assert_eq!(
            answer.header("access-control-expose-headers"),
            None,
            "{wiring}: a preflight doesn't expose"
        );
    }
}

#[tokio::test]
async fn no_route_means_no_cors_headers() {
    let policy = CorsPolicy::exposing("X-Total-Count");
    for (wiring, answer) in each(&policy, "GET", "/no-such-path", &[ORIGIN]).await {
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{wiring}");
        assert_eq!(
            answer.header("access-control-allow-origin"),
            None,
            "{wiring}"
        );
        assert_eq!(
            answer.header("access-control-expose-headers"),
            None,
            "{wiring}"
        );
    }
    for (wiring, options) in each(&policy, "OPTIONS", "/no-such-path", &[]).await {
        assert_eq!(options.status, StatusCode::NOT_FOUND, "{wiring}");
        assert_eq!(options.header("vary"), None, "{wiring}");
    }
}

#[tokio::test]
async fn an_options_request_that_isnt_a_preflight_is_routed_and_gets_vary() {
    let policy = CorsPolicy::default();
    for (wiring, with_origin) in each(&policy, "OPTIONS", "/things", &[ORIGIN]).await {
        assert_eq!(
            with_origin.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{wiring}"
        );
        assert_eq!(with_origin.header("vary"), Some(VARY), "{wiring}");
        assert_eq!(
            with_origin.header("access-control-allow-origin"),
            Some("*"),
            "{wiring}"
        );
    }
    // A method that isn't a token isn't a preflight either.
    let bad = [ORIGIN, ("access-control-request-method", "G T")];
    for (wiring, bad_method) in each(&policy, "OPTIONS", "/things", &bad).await {
        assert_eq!(
            bad_method.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{wiring}"
        );
    }
    let no_origin = [("access-control-request-method", "GET")];
    for (wiring, without_origin) in each(&policy, "OPTIONS", "/things", &no_origin).await {
        assert_eq!(
            without_origin.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{wiring}"
        );
        assert_eq!(without_origin.header("vary"), Some(VARY), "{wiring}");
        assert_eq!(
            without_origin.header("access-control-allow-origin"),
            None,
            "{wiring}"
        );
    }
}

#[tokio::test]
async fn a_wrong_method_is_tapirs_405_without_allow() {
    // captures http/emails-post.txt, health-head.txt
    for method in ["POST", "PUT", "PATCH", "DELETE", "OPTIONS", "TRACE", "HEAD"] {
        for path in ["/things", "/plain"] {
            for (wiring, answer) in each(&CorsPolicy::default(), method, path, &[]).await {
                let what = format!("{wiring}: {method} {path}");
                assert_eq!(answer.status, StatusCode::METHOD_NOT_ALLOWED, "{what}");
                assert_eq!(answer.header("content-type"), Some(TEXT), "{what}");
                assert_eq!(answer.header("allow"), None, "{what}");
                let body = if method == "HEAD" {
                    ""
                } else {
                    "Method Not Allowed"
                };
                assert_eq!(answer.body, body, "{what}");
            }
        }
    }
}

#[test]
fn the_policy_allows_its_five_methods_and_content_type_only() {
    use axum::http::Method;
    let policy = CorsPolicy::default();
    let content_type = vec!["content-type".to_owned()];
    assert!(policy.allows(&Method::GET, &content_type));
    assert!(policy.allows(&Method::DELETE, &[]));
    assert!(!policy.allows(&Method::HEAD, &[]));
    assert!(!policy.allows(&Method::GET, &["authorization".to_owned()]));
    assert!(!policy.allows(&Method::GET, &[String::new()]));
}

#[test]
fn a_policy_may_allow_authorization_too() {
    // inventory, fulfilment, payment and order-service's Cors.scala (all-Rust Stage A)
    use axum::http::Method;
    let policy = CorsPolicy::default().allowing(&["Content-Type", "Authorization"]);
    let both = vec!["authorization".to_owned(), "Content-Type".to_owned()];
    assert!(policy.allows(&Method::GET, &both));
    assert!(!policy.allows(&Method::GET, &["x-other".to_owned()]));
}

#[tokio::test]
async fn a_preflight_lists_the_policys_headers() {
    let policy = CorsPolicy::default().allowing(&["Content-Type", "Authorization"]);
    let headers = [
        ORIGIN,
        ("access-control-request-method", "POST"),
        (
            "access-control-request-headers",
            "authorization, content-type",
        ),
    ];
    for (wiring, answer) in each(&policy, "OPTIONS", "/things", &headers).await {
        assert_eq!(answer.status, StatusCode::OK, "{wiring}");
        assert_eq!(
            answer.header("access-control-allow-headers"),
            Some("Content-Type, Authorization"),
            "{wiring}"
        );
    }
}

#[test]
fn header_lists_split_as_java_splits_them() {
    assert_eq!(split_names("Content-Type"), ["Content-Type"]);
    assert_eq!(split_names(" a , b ,, "), ["a", "b"]);
    assert_eq!(split_names("a,"), ["a"]);
    assert_eq!(split_names(""), [""]);
}
