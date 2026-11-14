// SPDX-License-Identifier: MIT

//! A minimal service on service-basics, for service-testing's `Service` tests:
//! the health routes, `POST /echo` and `PUT /redirect`. Prints
//! `$FIXTURE_GREETING on port N`, and `fixture: stopping` on SIGTERM.

use std::net::{Ipv4Addr, SocketAddr};
use std::process::ExitCode;

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{post, put};
use service_basics::health::{self, Readiness};
use service_basics::text_error::{method_not_allowed, not_found};
use service_basics::{config, environment, runtime, shutdown};

/// Always ready: the fixture has no dependencies.
#[derive(Clone)]
struct Up;

impl Readiness for Up {
    fn ready(&self) -> impl Future<Output = bool> + Send {
        std::future::ready(true)
    }
}

fn main() -> ExitCode {
    let env = environment::variables();
    let mut problems = Vec::new();
    let port = config::port(&env, "FIXTURE_PORT", 8080, &mut problems);
    let Some(greeting) = env.get("FIXTURE_GREETING").cloned() else {
        eprintln!("fixture: FIXTURE_GREETING is not set");
        return ExitCode::FAILURE;
    };
    if !problems.is_empty() {
        eprintln!("fixture: {}", config::ConfigError { problems });
        return ExitCode::FAILURE;
    }
    let Ok(runtime) = runtime::multi_thread() else {
        eprintln!("fixture: can't start the async runtime");
        return ExitCode::FAILURE;
    };
    runtime.block_on(async {
        let Ok(listener) =
            tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))).await
        else {
            eprintln!("fixture: can't bind port {port}");
            return ExitCode::FAILURE;
        };
        println!("{greeting} on port {port}");
        let app = health::routes()
            .route("/echo", post(echo))
            .route("/redirect", put(redirect))
            .method_not_allowed_fallback(method_not_allowed)
            .fallback(not_found)
            .with_state(Up);
        let served = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown::signal("fixture"))
            .await;
        eprintln!("fixture: stopping");
        if served.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    })
}

/// The body back, with the request's Content-Type if it had one.
async fn echo(headers: HeaderMap, body: String) -> Response {
    if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        ([(header::CONTENT_TYPE, content_type.clone())], body).into_response()
    } else {
        let mut response = body.into_response();
        response.headers_mut().remove(header::CONTENT_TYPE);
        response
    }
}

/// 303 to the `x-to` header's value.
async fn redirect(headers: HeaderMap) -> Response {
    if let Some(location) = headers.get("x-to").cloned() {
        (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
    } else {
        StatusCode::BAD_REQUEST.into_response()
    }
}
