// SPDX-License-Identifier: MIT

//! service-testing's `Service` against the fixture binary (`src/bin/fixture.rs`),
//! which also proves `shutdown::signal`: SIGTERM stops it with exit 0.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use service_testing::Service;

const FIXTURE: &str = env!("CARGO_BIN_EXE_fixture");

fn start() -> Service {
    Service::start(
        FIXTURE,
        "FIXTURE_PORT",
        &[("FIXTURE_GREETING", "hello".to_owned())],
    )
}

#[test]
fn start_waits_until_health_answers() {
    let service = start();
    assert_eq!(service.get("/health"), Some((200, None, String::new())));
}

#[test]
fn sigterm_stops_it_with_exit_zero_and_its_output_is_kept() {
    let mut service = start();
    service.terminate();
    assert!(service.wait_exit().success());
    assert_eq!(
        service.stdout(),
        format!("hello on port {}\n", service.port)
    );
    assert_eq!(service.stderr(), "fixture: stopping\n");
}

#[test]
fn it_runs_with_only_the_environment_it_is_given() {
    let mut service = Service::spawn(FIXTURE, "FIXTURE_PORT", &[]);
    assert!(!service.wait_exit().success());
    assert_eq!(service.stderr(), "fixture: FIXTURE_GREETING is not set\n");
}

#[test]
fn request_returns_every_header_and_head_has_no_body() {
    let service = start();
    let (status, headers, body) = service.request("HEAD", "/health", &[]);
    assert_eq!((status, body.as_str()), (405, ""));
    assert!(headers.contains(&(
        "content-type".to_owned(),
        "text/plain; charset=UTF-8".to_owned()
    )));
}

#[test]
fn post_sends_its_body_and_content_type() {
    let service = start();
    assert_eq!(
        service.post_json("/echo", "{\"a\":1}"),
        (
            200,
            Some("application/json".to_owned()),
            "{\"a\":1}".to_owned()
        )
    );
    assert_eq!(
        service.post("/echo", b"raw", None),
        (200, None, "raw".to_owned())
    );
}

#[test]
fn request_with_body_sends_headers_and_does_not_follow_redirects() {
    let service = start();
    let (status, headers, _) =
        service.request_with_body("PUT", "/redirect", &[("x-to", "/health")], "");
    assert_eq!(status, 303);
    assert!(headers.contains(&("location".to_owned(), "/health".to_owned())));
}

#[test]
fn get_is_none_when_nothing_answers() {
    let mut service = start();
    service.terminate();
    service.wait_exit();
    assert_eq!(service.get("/health"), None);
}
