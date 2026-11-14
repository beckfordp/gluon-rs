// SPDX-License-Identifier: MIT

//! The pool and the bounded readiness ping, against a throwaway cluster.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use service_basics::config::PostgresConfig;
use service_basics::postgres::{PingError, connect, ping};
use service_testing::{Postgres, free_port};

fn config(port: u16) -> PostgresConfig {
    PostgresConfig {
        host: "127.0.0.1".to_owned(),
        port,
        database: "user".to_owned(),
        user: "user".to_owned(),
        password: "user".to_owned(),
    }
}

#[tokio::test]
async fn a_connected_pool_has_the_services_size_and_answers_a_ping() {
    let postgres = Postgres::start();
    let pool = connect(&config(postgres.port), 7).await.unwrap();
    assert_eq!(pool.options().get_max_connections(), 7);
    assert!(ping(&pool, Duration::from_secs(2)).await.is_ok());
}

#[tokio::test]
async fn nothing_listening_fails_at_connect() {
    assert!(connect(&config(free_port()), 10).await.is_err());
}

#[tokio::test]
async fn a_wrong_password_fails_at_connect() {
    let postgres = Postgres::start();
    let wrong = PostgresConfig {
        password: "nope".to_owned(),
        ..config(postgres.port)
    };
    assert!(connect(&wrong, 10).await.is_err());
}

#[tokio::test]
async fn a_paused_database_is_a_timeout_not_a_hang() {
    let postgres = Postgres::start();
    let pool = connect(&config(postgres.port), 10).await.unwrap();
    postgres.pause();
    let started = Instant::now();
    let result = ping(&pool, Duration::from_millis(300)).await;
    assert!(
        matches!(result, Err(PingError::Timeout(within)) if within == Duration::from_millis(300)),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    postgres.resume();
}

#[tokio::test]
async fn a_stopped_database_is_an_error() {
    let postgres = Postgres::start();
    let pool = connect(&config(postgres.port), 10).await.unwrap();
    postgres.stop();
    let result = ping(&pool, Duration::from_secs(5)).await;
    assert!(result.is_err(), "{result:?}");
}

#[test]
fn a_timeout_says_how_long_it_waited() {
    assert_eq!(
        PingError::Timeout(Duration::from_secs(2)).to_string(),
        "no answer within 2s"
    );
}
