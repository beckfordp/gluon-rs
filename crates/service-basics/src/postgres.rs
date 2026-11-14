// SPDX-License-Identifier: MIT

//! The service's Postgres pool and the bounded readiness ping. Each service
//! keeps its own migrations, and passes its pool size, citing its Scala source.

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::config::PostgresConfig;

/// How long a request waits for a connection before failing.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);

/// Why a ping failed.
#[derive(Debug, thiserror::Error)]
pub enum PingError {
    /// Postgres didn't answer in time (paused, or overloaded).
    #[error("no answer within {0:?}")]
    Timeout(Duration),
    /// No connection, or the query failed.
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
}

/// Connects a pool of `pool_size` (one connection is opened now, so a wrong
/// address fails at startup).
///
/// # Errors
/// When the database can't be reached or refuses the login.
pub async fn connect(config: &PostgresConfig, pool_size: u32) -> Result<PgPool, sqlx::Error> {
    let options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .database(&config.database)
        .username(&config.user)
        .password(&config.password);
    PgPoolOptions::new()
        .max_connections(pool_size)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await
}

/// `SELECT 1` within `within`. Bounded, so a paused database is a quick
/// [`PingError::Timeout`], not a hang (tests/contract implicit behaviour N1).
///
/// # Errors
/// [`PingError`]: no answer in time, or no connection.
pub async fn ping(pool: &PgPool, within: Duration) -> Result<(), PingError> {
    let work = async {
        let mut conn = pool.acquire().await?;
        sqlx::query("SELECT 1").execute(&mut *conn).await?;
        Ok(())
    };
    tokio::time::timeout(within, work)
        .await
        .unwrap_or(Err(PingError::Timeout(within)))
}
