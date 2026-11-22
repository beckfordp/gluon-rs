// SPDX-License-Identifier: MIT

//! What a Rust port's integration tests need (gluon TD-43): a throwaway
//! PostgreSQL 16 cluster and Redis server from the local binaries (no Docker in
//! the sandbox) and the service binary under test.
//!
//! Test code only: a helper that can't do its job panics, which is how a test
//! fails (`rust.md` rule 2), so no method documents its panics.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc
)]

mod postgres;
mod redis;
mod service;

pub use postgres::Postgres;
pub use redis::Redis;
pub use service::Service;

use std::net::TcpListener;

/// A port nothing listens on right now.
#[must_use]
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
