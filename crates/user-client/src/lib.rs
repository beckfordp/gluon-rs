// SPDX-License-Identifier: MIT

//! The user-service lookup (ADR 0025; notification-service `UserClient`):
//! `GET {base}/users/{id}` through purerest's resilience, traced and metered
//! (`telemetry::client`). Moved from user-service-rs's events-probe so
//! notification-service-rs's consumers share it (gluon TD-59).

use std::time::Duration;

use serde::Deserialize;
use telemetry::client::{ClientError, ResilientClient, TracedClient};
use telemetry::metrics::Metrics;
use telemetry::resilience::{CircuitBreakerConfig, RetryConfig};
use telemetry::tracing::SdkTracer;

/// Why a lookup failed: the caller retries it, then dead-letters it.
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    /// No response (breaker open, transport).
    #[error(transparent)]
    Client(#[from] ClientError),
    /// A 200 whose body isn't a user.
    #[error(transparent)]
    Body(reqwest::Error),
    /// Neither 200 nor 400/404.
    #[error("Unexpected response from user-service for {user_id}: {status}")]
    Status {
        /// Whom it was asked about.
        user_id: String,
        /// What it answered.
        status: u16,
    },
}

#[derive(Debug, Deserialize)]
struct UserBody {
    email: String,
}

/// The lookup: request timeout, client span and metrics, circuit breaker,
/// retry.
#[derive(Debug, Clone)]
pub struct UserClient {
    client: ResilientClient,
    base_url: String,
}

impl UserClient {
    /// Calls `base_url` with these settings.
    ///
    /// # Errors
    /// When the HTTP client can't be built.
    pub fn new(
        base_url: &str,
        timeout: Duration,
        retry: RetryConfig,
        breaker: CircuitBreakerConfig,
        tracer: SdkTracer,
        metrics: &Metrics,
    ) -> Result<Self, reqwest::Error> {
        let instrumented = TracedClient::new(timeout, tracer, metrics)?;
        Ok(Self::from_client(
            base_url,
            ResilientClient::new(instrumented, retry, breaker, metrics),
        ))
    }

    /// Calls `base_url` through `client`.
    #[must_use]
    pub fn from_client(base_url: &str, client: ResilientClient) -> Self {
        Self {
            client,
            base_url: base_url.to_owned(),
        }
    }

    /// The email of `user_id`: `Some` when found, `None` when user-service
    /// doesn't know it or can't read it (404, 400: permanently unresolved).
    ///
    /// # Errors
    /// Any other answer, or none: worth retrying.
    pub async fn email(&self, user_id: &str) -> Result<Option<String>, LookupError> {
        let url = format!("{}/users/{user_id}", self.base_url);
        let response = self.client.get(&url).await?;
        match response.status().as_u16() {
            200 => response
                .json::<UserBody>()
                .await
                .map(|body| Some(body.email))
                .map_err(LookupError::Body),
            400 | 404 => Ok(None),
            status => Err(LookupError::Status {
                user_id: user_id.to_owned(),
                status,
            }),
        }
    }
}
