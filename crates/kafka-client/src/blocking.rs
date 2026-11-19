// SPDX-License-Identifier: MIT

//! Synchronous librdkafka calls made from async code (gluon TD-64.3).

use tokio::runtime::{Handle, RuntimeFlavor};

/// Runs `work`, which blocks, without stalling the async worker that called it:
/// inside `block_in_place` on a multi-thread runtime (the worker's other tasks
/// move to another thread first); inline on a current-thread runtime or none,
/// where there is no other thread to move them to.
pub fn off_worker<T>(work: impl FnOnce() -> T) -> T {
    match Handle::try_current().map(|handle| handle.runtime_flavor()) {
        Ok(RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}
