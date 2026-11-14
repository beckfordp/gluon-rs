// SPDX-License-Identifier: MIT

//! The async runtime every service's `main` starts.

use std::io;

use tokio::runtime::{Builder, Runtime};

/// tokio's multi-threaded runtime, with I/O and timers.
///
/// # Errors
/// When the runtime can't be started.
pub fn multi_thread() -> io::Result<Runtime> {
    Builder::new_multi_thread().enable_all().build()
}
