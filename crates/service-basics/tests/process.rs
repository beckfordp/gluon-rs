// SPDX-License-Identifier: MIT

//! The process pieces every `main` shares: the environment map and the
//! runtime. Shutdown is tested through service-testing's fixture binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use service_basics::{environment, runtime};

#[test]
fn the_environment_keeps_utf8_variables_and_drops_the_rest() {
    let vars = [
        (OsString::from("KEPT"), OsString::from("value")),
        (OsString::from("BAD_VALUE"), OsString::from_vec(vec![0xff])),
        (OsString::from_vec(vec![0xfe]), OsString::from("bad name")),
    ];
    let env = environment::from_os(vars);
    assert_eq!(env.len(), 1);
    assert_eq!(env.get("KEPT").map(String::as_str), Some("value"));
}

#[test]
fn the_process_environment_is_read() {
    let env = environment::variables();
    assert_eq!(env.get("PATH"), std::env::var("PATH").ok().as_ref());
}

#[test]
fn the_runtime_runs_futures_on_several_threads() {
    let runtime = runtime::multi_thread().unwrap();
    let answer = runtime.block_on(async { tokio::spawn(async { 42 }).await.unwrap() });
    assert_eq!(answer, 42);
    assert!(runtime.metrics().num_workers() >= 1);
}
