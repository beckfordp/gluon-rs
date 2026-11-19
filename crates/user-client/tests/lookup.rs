// SPDX-License-Identifier: MIT

//! `GET {base}/users/{id}` as notification-service's `UserClient` reads the
//! answer (ADR 0025): 200 is the user's email, 400 and 404 mean no such user
//! (permanently unresolved), anything else fails for the DLQ guard to retry.
//! Against a local server; the retry and breaker themselves are telemetry's,
//! tested there.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry_sdk::trace::InMemorySpanExporter;
use telemetry::client::{ClientError, ResilientClient, TracedClient};
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};
use telemetry::tracing::Tracing;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use user_client::{LookupError, UserClient};

const USER: &str = "11111111-1111-4111-8111-111111111111";

/// A local server answering every request `status` with `body`; records each
/// request line.
async fn server(status: u16, body: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let log = seen.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let log = log.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0; 4096];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buffer[..read]).into_owned();
                log.lock()
                    .unwrap()
                    .push(head.lines().next().unwrap_or_default().to_owned());
                let response = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    (url, seen)
}

/// A client on `base_url` retrying once, quickly, its breaker opening after
/// `threshold` failures.
fn client(base_url: &str, threshold: u32) -> UserClient {
    let metrics = Metrics::new("user-client");
    let tracing = Tracing::with_exporter("user-client", 1.0, InMemorySpanExporter::default());
    let resilient = ResilientClient::new(
        TracedClient::new(Duration::from_secs(2), tracing.tracer(), &metrics).unwrap(),
        RetryConfig {
            max_retries: 1,
            base_delay: Duration::from_millis(5),
        },
        CircuitBreakerConfig {
            failure_threshold: threshold,
            reset_timeout: Duration::from_secs(60),
        },
        &metrics,
    );
    UserClient::from_client(base_url, resilient)
}

#[tokio::test]
async fn a_found_user_is_their_email_from_get_users_id() {
    let (url, seen) = server(
        200,
        r#"{"userId":"u","email":"ada@example.com","createdAt":"x"}"#,
    )
    .await;
    let email = client(&url, 5).email(USER).await.unwrap();
    assert_eq!(email.as_deref(), Some("ada@example.com"));
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [format!("GET /users/{USER} HTTP/1.1")]
    );
}

#[tokio::test]
async fn an_unknown_or_malformed_id_is_no_user() {
    for status in [404, 400] {
        let (url, seen) = server(status, "").await;
        assert_eq!(client(&url, 5).email(USER).await.unwrap(), None, "{status}");
        assert_eq!(seen.lock().unwrap().len(), 1, "{status}: not retried");
    }
}

#[tokio::test]
async fn any_other_status_fails_with_the_users_id_and_the_status() {
    for (status, asked) in [(503, 2), (409, 1)] {
        let (url, seen) = server(status, "").await;
        let error = client(&url, 5).email(USER).await.unwrap_err();
        let LookupError::Status {
            user_id,
            status: got,
        } = &error
        else {
            panic!("{status}: want Status, got {error:?}");
        };
        assert_eq!((user_id.as_str(), *got), (USER, status));
        assert_eq!(
            error.to_string(),
            format!("Unexpected response from user-service for {USER}: {status}")
        );
        assert_eq!(
            seen.lock().unwrap().len(),
            asked,
            "{status}: a 5xx is retried"
        );
    }
}

#[tokio::test]
async fn a_body_that_isnt_a_user_fails() {
    let (url, _) = server(200, r#"{"userId":"u"}"#).await;
    let error = client(&url, 5).email(USER).await.unwrap_err();
    assert!(matches!(error, LookupError::Body(_)), "{error:?}");
}

#[tokio::test]
async fn no_answer_fails_with_the_clients_error() {
    let unused = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", unused.local_addr().unwrap());
    drop(unused);
    let error = client(&url, 5).email(USER).await.unwrap_err();
    assert!(
        matches!(error, LookupError::Client(ClientError::Transport(_))),
        "{error:?}"
    );
    let open = client(&url, 1);
    let _ = open.email(USER).await;
    let error = open.email(USER).await.unwrap_err();
    assert!(
        matches!(error, LookupError::Client(ClientError::Open)),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "Circuit breaker is open");
}

#[test]
fn the_service_constructor_builds_its_own_client() {
    let metrics = Metrics::new("user-client");
    let tracing = Tracing::with_exporter("user-client", 1.0, InMemorySpanExporter::default());
    let built = UserClient::new(
        "http://user-service:8084",
        Duration::from_secs(5),
        RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
        },
        CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
        },
        tracing.tracer(),
        &metrics,
    );
    assert!(built.is_ok());
}
