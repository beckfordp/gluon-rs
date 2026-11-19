// SPDX-License-Identifier: MIT

//! What "now" is for a consumer: the wall clock in a service, a stopped one in
//! tests (gluon TD-64.2), so a record's age is testable without a broker.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time.
pub trait Clock: Send + Sync {
    /// Now.
    fn now(&self) -> SystemTime;
}

/// The wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// `time` as milliseconds since the epoch; 0 before it, `i64::MAX` past it.
#[must_use]
pub fn epoch_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| {
        i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
    })
}
