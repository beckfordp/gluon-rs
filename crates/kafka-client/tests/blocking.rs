// SPDX-License-Identifier: MIT

//! The per-record synchronous commit runs off the async worker (gluon TD-64.3):
//! inside `block_in_place` on a multi-thread runtime, so the worker's other tasks
//! move on; inline on a current-thread runtime, where `block_in_place` would panic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kafka_client::blocking::off_worker;
use tokio::runtime::{Builder, Handle};

#[test]
fn on_a_multi_thread_runtime_the_work_leaves_the_worker() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();
    let got = runtime.block_on(async {
        // Blocking on the runtime from a worker panics unless block_in_place
        // has handed the worker's tasks to another thread first.
        off_worker(|| Handle::current().block_on(async { 7 }))
    });
    assert_eq!(got, 7);
}

#[test]
fn on_a_current_thread_runtime_the_work_runs_inline() {
    let runtime = Builder::new_current_thread().build().unwrap();
    assert_eq!(runtime.block_on(async { off_worker(|| 2) }), 2);
}
