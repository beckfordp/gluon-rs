// SPDX-License-Identifier: MIT

//! The cache against a real Redis (service-testing's): round trips, expiry, and
//! Redis down then back (gluon all-Rust Stage A P0.3; the sandbox's client
//! spent 19 s per request reconnecting while Redis was down).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use redis_cache::{Cache, CacheError};
use service_testing::Redis;

const TIMEOUT: Duration = Duration::from_millis(200);

#[tokio::test]
async fn a_value_set_is_got_until_deleted() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    assert_eq!(cache.get("cart:1").await.unwrap(), None);
    cache
        .set("cart:1", "{\"lines\":[]}", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        cache.get("cart:1").await.unwrap().as_deref(),
        Some("{\"lines\":[]}")
    );
    cache.delete(&["cart:1", "cart:2"]).await.unwrap();
    assert_eq!(cache.get("cart:1").await.unwrap(), None);
    assert!(cache.ping().await);
}

#[tokio::test]
async fn a_value_expires_after_its_ttl() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    cache.set("k", "v", Duration::from_secs(1)).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while cache.get("k").await.unwrap().is_some() {
        assert!(
            Instant::now() < deadline,
            "still cached 10 s after a 1 s ttl"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn while_redis_is_down_every_call_fails_at_once_and_then_recovers() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    cache.set("k", "v", Duration::from_secs(60)).await.unwrap();
    redis.stop();
    for _ in 0..5 {
        let started = Instant::now();
        let result = cache.get("k").await;
        assert!(
            matches!(result, Err(CacheError::Unavailable(_))),
            "{result:?}"
        );
        // A bound, not a measurement: a reconnect with retries takes seconds.
        assert!(
            started.elapsed() < TIMEOUT * 5,
            "took {:?}",
            started.elapsed()
        );
    }
    assert!(!cache.ping().await);
    redis.start_again();
    let deadline = Instant::now() + Duration::from_secs(10);
    while cache
        .set("k", "back", Duration::from_secs(60))
        .await
        .is_err()
    {
        assert!(
            Instant::now() < deadline,
            "no reconnect 10 s after Redis came back"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(cache.get("k").await.unwrap().as_deref(), Some("back"));
}

#[test]
fn a_bad_url_is_refused_up_front() {
    assert!(matches!(
        Cache::new("http://x", TIMEOUT),
        Err(CacheError::Url(_))
    ));
}

// --- cache-aside over the source of truth --------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Product {
    id: String,
    price: i64,
}

fn product(price: i64) -> Product {
    Product {
        id: "p-1".to_owned(),
        price,
    }
}

#[tokio::test]
async fn a_miss_loads_and_caches_then_a_hit_does_not_load() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    let loads = std::sync::atomic::AtomicUsize::new(0);
    let load = || async {
        loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok::<_, String>(product(100))
    };
    let ttl = Duration::from_secs(60);
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, load).await,
        Ok(product(100))
    );
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, load).await,
        Ok(product(100))
    );
    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 1);
    // The data changed; the cached answer stands until it is invalidated.
    let changed = || async { Ok::<_, String>(product(200)) };
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, changed).await,
        Ok(product(100))
    );
    cache.delete(&["product:p-1"]).await.unwrap();
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, changed).await,
        Ok(product(200))
    );
}

#[tokio::test]
async fn with_redis_down_every_read_loads_and_a_load_error_is_the_callers() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    redis.stop();
    let ttl = Duration::from_secs(60);
    for price in [1, 2] {
        let load = || async move { Ok::<_, String>(product(price)) };
        assert_eq!(
            cache.get_or_load("product:p-1", ttl, load).await,
            Ok(product(price))
        );
    }
    let failing = || async { Err::<Product, _>("postgres down".to_owned()) };
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, failing).await,
        Err("postgres down".to_owned())
    );
}

#[tokio::test]
async fn an_unreadable_cached_value_is_loaded_again() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    cache
        .set("product:p-1", "not json", Duration::from_secs(60))
        .await
        .unwrap();
    let load = || async { Ok::<_, String>(product(7)) };
    let ttl = Duration::from_secs(60);
    assert_eq!(
        cache.get_or_load("product:p-1", ttl, load).await,
        Ok(product(7))
    );
    let cached = cache.get("product:p-1").await.unwrap().unwrap();
    assert_eq!(
        serde_json::from_str::<Product>(&cached).unwrap(),
        product(7)
    );
}

// --- any command, bounded the same way (all-Rust Stage A, cart: its store is Redis) --------

#[tokio::test]
async fn any_command_runs_bounded_and_a_script_runs_atomically() {
    let redis = Redis::start();
    let cache = Cache::new(&redis.url(), TIMEOUT).unwrap();
    let mut hset = redis_cache::redis::cmd("HSET");
    hset.arg("cart:1").arg("a").arg("2");
    let added: i64 = cache.query(&hset).await.unwrap();
    assert_eq!(added, 1);
    let mut eval = redis_cache::redis::cmd("EVAL");
    eval.arg("redis.call('HINCRBY', KEYS[1], 'a', ARGV[1]) return redis.call('HGETALL', KEYS[1])")
        .arg(1)
        .arg("cart:1")
        .arg(3);
    let fields: Vec<String> = cache.query(&eval).await.unwrap();
    assert_eq!(fields, ["a", "5"]);
    redis.stop();
    let mut get = redis_cache::redis::cmd("HGETALL");
    get.arg("cart:1");
    assert!(matches!(
        cache.query::<Vec<String>>(&get).await,
        Err(CacheError::Unavailable(_))
    ));
}
