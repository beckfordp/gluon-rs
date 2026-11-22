// SPDX-License-Identifier: MIT

//! A Redis client for the Rust ports: cache-aside reads ([`Cache::get_or_load`]) and,
//! for cart-service, its store ([`Cache::query`]) (cart, catalog and
//! order-service cache in Redis in front of Postgres; gluon all-Rust Stage A
//! P0.3).
//!
//! A cache is never the source of truth: a caller treats any error as a miss
//! and reads Postgres. So the client must fail fast while Redis is down — the
//! sandbox's first client reconnected with retries inside every command and
//! spent 19 s per request. Here one connection is shared; every call is bounded
//! by the timeout given (connecting included); a failed call drops the
//! connection and the next call connects afresh, once, so a Redis that comes
//! back is used again on the next call. (Not redis's `ConnectionManager`: its
//! reconnect retries are what made the sandbox's requests slow.)

use std::future::Future;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The client this wraps, for building commands ([`Cache::query`]).
pub use redis;
use redis::aio::MultiplexedConnection;
use redis::{AsyncConnectionConfig, Client, RedisError};
use tokio::sync::Mutex;

/// Why a cache call failed: the caller reads the source of truth instead.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// The URL isn't a Redis one.
    #[error("not a Redis URL: {0}")]
    Url(#[source] RedisError),
    /// Redis didn't answer: down, unreachable, or slower than the timeout.
    #[error("Redis unavailable: {0}")]
    Unavailable(String),
}

/// The cache: one Redis, one shared connection, each call bounded.
pub struct Cache {
    client: Client,
    timeout: Duration,
    connection: Mutex<Option<MultiplexedConnection>>,
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl Cache {
    /// A cache on `url` (`redis://host:port`), each call bounded by `timeout`.
    /// Nothing connects until the first call.
    ///
    /// # Errors
    /// [`CacheError::Url`] for a URL Redis can't use.
    pub fn new(url: &str, timeout: Duration) -> Result<Self, CacheError> {
        Ok(Self {
            client: Client::open(url).map_err(CacheError::Url)?,
            timeout,
            connection: Mutex::new(None),
        })
    }

    /// The value at `key`, if any.
    ///
    /// # Errors
    /// [`CacheError::Unavailable`]: treat as a miss.
    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        self.run(redis::cmd("GET").arg(key)).await
    }

    /// Sets `key` to `value` for `ttl` (at least a second, Redis's `EX`).
    ///
    /// # Errors
    /// [`CacheError::Unavailable`]: the value isn't cached.
    pub async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError> {
        let seconds = ttl.as_secs().max(1);
        self.run(redis::cmd("SET").arg(key).arg(value).arg("EX").arg(seconds))
            .await
    }

    /// Deletes `keys` (an absent one is no error).
    ///
    /// # Errors
    /// [`CacheError::Unavailable`]: the keys may still be cached.
    pub async fn delete(&self, keys: &[&str]) -> Result<(), CacheError> {
        if keys.is_empty() {
            return Ok(());
        }
        self.run::<i64>(redis::cmd("DEL").arg(keys))
            .await
            .map(|_| ())
    }

    /// Cache-aside: the value cached at `key`, else `load`'s (the source of
    /// truth, e.g. Postgres), cached for `ttl`. Redis down, or a cached value
    /// that doesn't read as `T`, is a miss: `load` answers. A cached value
    /// stands until it expires or the caller deletes it after a change.
    ///
    /// # Errors
    /// `load`'s error; a cache error never is.
    pub async fn get_or_load<T, E, F, Fut>(&self, key: &str, ttl: Duration, load: F) -> Result<T, E>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        if let Ok(Some(cached)) = self.get(key).await
            && let Ok(value) = serde_json::from_str(&cached)
        {
            return Ok(value);
        }
        let value = load().await?;
        if let Ok(json) = serde_json::to_string(&value) {
            let _ = self.set(key, &json, ttl).await;
        }
        Ok(value)
    }

    /// Any command (a script's `EVAL`, `HGETALL`, ...), bounded and reconnected
    /// like the others: for a service whose store is Redis (cart-service).
    ///
    /// # Errors
    /// [`CacheError::Unavailable`]: Redis didn't answer, or refused the
    /// command, or its reply isn't a `T`.
    pub async fn query<T: redis::FromRedisValue>(
        &self,
        command: &redis::Cmd,
    ) -> Result<T, CacheError> {
        self.run(command).await
    }

    /// Whether Redis answers `PING` within the timeout.
    pub async fn ping(&self) -> bool {
        self.run::<String>(&redis::cmd("PING")).await.is_ok()
    }

    async fn run<T: redis::FromRedisValue>(&self, command: &redis::Cmd) -> Result<T, CacheError> {
        let work = async {
            let mut connection = self.connection().await?;
            command.query_async::<T>(&mut connection).await
        };
        let error = match tokio::time::timeout(self.timeout, work).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(error)) => error.to_string(),
            Err(_) => format!("no answer within {:?}", self.timeout),
        };
        // The next call connects afresh: a broken connection isn't reused.
        *self.connection.lock().await = None;
        Err(CacheError::Unavailable(error))
    }

    /// The shared connection, opened if there is none (the lock is held only
    /// while opening: queries run concurrently on the multiplexed connection).
    async fn connection(&self) -> Result<MultiplexedConnection, RedisError> {
        let mut slot = self.connection.lock().await;
        if let Some(open) = slot.as_ref() {
            return Ok(open.clone());
        }
        let config = AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.timeout))
            .set_response_timeout(Some(self.timeout));
        let open = self
            .client
            .get_multiplexed_async_connection_with_config(&config)
            .await?;
        *slot = Some(open.clone());
        Ok(open)
    }
}
