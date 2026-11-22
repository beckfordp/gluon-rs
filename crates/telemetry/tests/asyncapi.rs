// SPDX-License-Identifier: MIT

//! AsyncAPI as purekafka generates and serves it (RUST-4.2 spec FR1-FR2).
//! Sources: capture 20261109 asyncapi.json, http/asyncapi*.txt; purekafka
//! `AsyncApiDoc`, `Docs.routes`; user-service `AppRoutes` (static HTML).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, UNIX_EPOCH};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use schemars::JsonSchema;
use serde_json::json;
use telemetry::docs::asyncapi::{self, Channel};
use tower::ServiceExt;

/// New user signed up: POST /auth/signup succeeded.
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(title = "UserRegisteredEvent")]
#[allow(dead_code)]
struct UserRegistered {
    /// User UUID. Also the partition key.
    #[schemars(example = &"5b6c7d8e-9f0a-4b1c-8d2e-3f4a5b6c7d8e")]
    user_id: String,
    /// User's email address.
    #[schemars(example = &"jane@example.com")]
    email: String,
}

/// Something consumed.
#[derive(JsonSchema)]
#[allow(dead_code)]
struct OrderCancelled {
    /// The order.
    order_id: String,
}

/// The capture's `asyncapi.json` (gluon-contract contracts/user-service/20261109), unchanged.
const CAPTURED: &str = include_str!("fixtures/asyncapi-user-service.json");

#[test]
fn user_services_document_is_byte_identical_to_the_scala_one() {
    let document = asyncapi::document(
        "User Service",
        "1.0",
        &[Channel::publish_cdc::<UserRegistered>(
            "user-events",
            "UserRegistered",
        )],
    );
    assert_eq!(
        serde_json::to_string(&document).unwrap(),
        CAPTURED.trim_end()
    );
}

#[test]
fn sends_come_before_receives_and_a_topic_with_both_gets_two_operations() {
    let document = asyncapi::document(
        "S",
        "1.0",
        &[
            Channel::subscribe::<OrderCancelled>("order-events", "OrderCancelled"),
            Channel::publish_cdc::<UserRegistered>("user-events", "UserRegistered"),
            Channel::publish_cdc::<OrderCancelled>("order-events", "OrderCancelled"),
        ],
    );
    let operations: Vec<&String> = document["operations"].as_object().unwrap().keys().collect();
    // AsyncApiDoc: topics in first-seen order, sends before receives.
    assert_eq!(
        operations,
        ["order-events-send", "user-events", "order-events-receive"]
    );
    assert_eq!(
        document["operations"]["order-events-receive"]["action"],
        "receive"
    );
    let channels: Vec<&String> = document["channels"].as_object().unwrap().keys().collect();
    assert_eq!(channels, ["order-events", "user-events"]);
}

#[test]
fn the_payload_schema_is_tapirs_open_2020_12_one() {
    let schema = asyncapi::payload_schema::<UserRegistered>();
    assert_eq!(
        schema["$schema"],
        "http://json-schema.org/draft/2020-12/schema#"
    );
    assert!(schema.get("additionalProperties").is_none());
    assert_eq!(
        schema.as_object().unwrap().keys().collect::<Vec<_>>(),
        [
            "$schema",
            "title",
            "description",
            "type",
            "required",
            "properties"
        ]
    );
}

fn app(dir: &std::path::Path) -> axum::Router {
    let document = json!({"asyncapi": "3.0.0"});
    asyncapi::router(&document, dir.to_path_buf())
}

async fn call(app: &axum::Router, method: &str, uri: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body(response: axum::response::Response) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

fn header(response: &axum::response::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .map(|v| v.to_str().unwrap().to_owned())
}

fn html_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("css")).unwrap();
    std::fs::write(dir.path().join("index.html"), "<html>page</html>").unwrap();
    std::fs::write(dir.path().join("css/a.css"), "body{}").unwrap();
    let stamp = UNIX_EPOCH + Duration::from_secs(1_762_713_786);
    for name in ["index.html", "css/a.css"] {
        let file = std::fs::File::options()
            .write(true)
            .open(dir.path().join(name))
            .unwrap();
        file.set_modified(stamp).unwrap();
    }
    dir
}

#[tokio::test]
async fn the_json_is_served_as_application_json() {
    let dir = html_dir();
    let response = call(&app(dir.path()), "GET", "/docs/asyncapi.json").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header(&response, "content-type").as_deref(),
        Some("application/json")
    );
    assert_eq!(body(response).await, br#"{"asyncapi":"3.0.0"}"#);
}

#[tokio::test]
async fn without_the_slash_it_redirects() {
    let dir = html_dir();
    let response = call(&app(dir.path()), "GET", "/docs/asyncapi").await;
    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        header(&response, "location").as_deref(),
        Some("/docs/asyncapi/")
    );
}

#[tokio::test]
async fn the_page_and_its_files_come_from_the_html_directory() {
    let dir = html_dir();
    for path in ["/docs/asyncapi/", "/docs/asyncapi/index.html"] {
        let response = call(&app(dir.path()), "GET", path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            header(&response, "content-type").as_deref(),
            Some("text/html")
        );
        assert_eq!(
            header(&response, "etag").as_deref(),
            Some(format!("\"{:x}-{:x}\"", 1_762_713_786_000_u64, 17).as_str())
        );
        assert_eq!(
            header(&response, "last-modified").as_deref(),
            Some("Sun, 09 Nov 2025 18:43:06 GMT")
        );
        assert_eq!(body(response).await, b"<html>page</html>");
    }
    let response = call(&app(dir.path()), "GET", "/docs/asyncapi/css/a.css").await;
    assert_eq!(
        header(&response, "content-type").as_deref(),
        Some("text/css")
    );
    assert_eq!(body(response).await, b"body{}");
}

#[tokio::test]
async fn anything_else_is_an_empty_404() {
    let dir = html_dir();
    for path in [
        "/docs/asyncapi/nope.js",
        "/docs/asyncapi/../Cargo.toml",
        "/docs/asyncapi/%2e%2e/secret",
        "/docs/asyncapi/css",
    ] {
        let response = call(&app(dir.path()), "GET", path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body(response).await, b"", "{path}");
    }
    let missing = tempfile::tempdir().unwrap();
    let gone = missing.path().join("never-generated");
    let response = call(&app(&gone), "GET", "/docs/asyncapi/").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn head_is_not_allowed() {
    let dir = html_dir();
    for path in [
        "/docs/asyncapi.json",
        "/docs/asyncapi/",
        "/docs/asyncapi/index.html",
    ] {
        let response = call(&app(dir.path()), "HEAD", path).await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
}

// --- all-Rust Stage A P0.3: a page built into the binary, no npm -----------------------------

fn two_way() -> serde_json::Value {
    asyncapi::document(
        "Order Service",
        "1.0",
        &[
            Channel::publish_cdc::<UserRegistered>("user-events", "UserRegistered"),
            Channel::subscribe::<OrderCancelled>("order-events", "OrderCancelled"),
        ],
    )
}

#[test]
fn the_page_lists_every_channel_its_direction_and_payload() {
    let page = asyncapi::page(&two_way());
    for wanted in [
        "<title>Order Service 1.0 — AsyncAPI</title>",
        "user-events",
        "publishes",
        "order-events",
        "consumes",
        "UserRegistered",
        "OrderCancelled",
        "&quot;userId&quot;",
    ] {
        assert!(page.contains(wanted), "{wanted} missing from\n{page}");
    }
    assert!(!page.contains("<script"), "no script needed: {page}");
}

#[test]
fn the_page_escapes_what_it_shows() {
    let document = json!({"asyncapi": "3.0.0", "info": {"title": "<b>x</b>", "version": "1"},
        "channels": {}, "operations": {}});
    let page = asyncapi::page(&document);
    assert!(page.contains("&lt;b&gt;x&lt;/b&gt;"), "{page}");
    assert!(!page.contains("<b>x</b>"), "{page}");
}

#[test]
fn a_service_without_events_publishes_and_consumes_nothing() {
    let document = asyncapi::document("Cart Service", "1.0", &[]);
    assert_eq!(document["channels"], json!({}));
    assert_eq!(document["operations"], json!({}));
    assert!(asyncapi::page(&document).contains("publishes and consumes nothing"));
}

#[tokio::test]
async fn the_built_in_page_is_served_with_the_json() {
    let app = asyncapi::router_with_page(&two_way());
    let page = call(&app, "GET", "/docs/asyncapi/").await;
    assert_eq!(page.status(), StatusCode::OK);
    assert_eq!(
        header(&page, "content-type").as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert!(
        String::from_utf8(body(page).await)
            .unwrap()
            .contains("order-events")
    );
    let index = call(&app, "GET", "/docs/asyncapi/index.html").await;
    assert_eq!(index.status(), StatusCode::OK);
    let json = call(&app, "GET", "/docs/asyncapi.json").await;
    assert_eq!(
        header(&json, "content-type").as_deref(),
        Some("application/json")
    );
    let served: serde_json::Value = serde_json::from_slice(&body(json).await).unwrap();
    assert_eq!(served, two_way());
    let redirect = call(&app, "GET", "/docs/asyncapi").await;
    assert_eq!(redirect.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        call(&app, "GET", "/docs/asyncapi/nope.js").await.status(),
        StatusCode::NOT_FOUND
    );
}
