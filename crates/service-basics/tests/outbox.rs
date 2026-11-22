// SPDX-License-Identifier: MIT

//! The outbox writer (gluon all-Rust Stage A P0.3): the shared envelope, written
//! in the caller's transaction to the table Debezium reads (ADR 0008).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde::Serialize;
use serde_json::json;
use service_basics::config::PostgresConfig;
use service_basics::outbox::{self, OutboxEvent};
use service_basics::postgres::connect;
use service_testing::Postgres;
use sqlx::Row;
use time::macros::datetime;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderConfirmed {
    order_id: String,
}

fn event() -> OutboxEvent {
    outbox::envelope(
        "OrderConfirmed",
        "order-service",
        "o-1",
        "corr-1",
        &OrderConfirmed {
            order_id: "o-1".to_owned(),
        },
        datetime!(2026-11-22 12:00:00.123_456_789 UTC),
    )
    .unwrap()
}

#[test]
fn the_envelope_is_the_shared_one() {
    let event = event();
    assert_eq!(event.aggregate_id, "o-1");
    assert_eq!(event.correlation_id, "corr-1");
    let envelope = &event.payload;
    let keys: Vec<&str> = envelope
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "eventId",
            "eventType",
            "timestamp",
            "source",
            "correlationId",
            "payload"
        ]
    );
    assert_eq!(envelope["eventType"], "OrderConfirmed");
    assert_eq!(envelope["source"], "order-service");
    assert_eq!(
        envelope["timestamp"], "2026-11-22T12:00:00.123456Z",
        "Java's clock: µs"
    );
    assert_eq!(envelope["payload"], json!({"orderId": "o-1"}));
    assert_ne!(envelope["eventId"].as_str().unwrap(), event.id.to_string());
}

async fn pool(postgres: &Postgres) -> sqlx::PgPool {
    let config = PostgresConfig {
        host: "127.0.0.1".to_owned(),
        port: postgres.port,
        database: "user".to_owned(),
        user: "user".to_owned(),
        password: "user".to_owned(),
    };
    let pool = connect(&config, 2).await.unwrap();
    sqlx::raw_sql(outbox::MIGRATION)
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn a_committed_write_is_one_row_as_debezium_reads_it() {
    let postgres = Postgres::start();
    let pool = pool(&postgres).await;
    let event = event();
    let mut tx = pool.begin().await.unwrap();
    outbox::write(&mut tx, &event, Some("00-abc-def-01"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let row = sqlx::query(
        "SELECT id, aggregate_id, payload, trace_parent, correlation_id FROM outbox_events",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<uuid::Uuid, _>("id"), event.id);
    assert_eq!(row.get::<String, _>("aggregate_id"), "o-1");
    assert_eq!(row.get::<serde_json::Value, _>("payload"), event.payload);
    assert_eq!(
        row.get::<Option<String>, _>("trace_parent").as_deref(),
        Some("00-abc-def-01")
    );
    assert_eq!(row.get::<String, _>("correlation_id"), "corr-1");
}

#[tokio::test]
async fn a_rolled_back_write_leaves_nothing() {
    let postgres = Postgres::start();
    let pool = pool(&postgres).await;
    let mut tx = pool.begin().await.unwrap();
    outbox::write(&mut tx, &event(), None).await.unwrap();
    tx.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
