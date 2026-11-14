// SPDX-License-Identifier: MIT

//! When to stop serving.

/// Completes on SIGTERM (how the harness and Kubernetes stop a process) or
/// SIGINT. If SIGTERM can't be watched, says so on stderr, prefixed with
/// `service`, and waits for SIGINT alone.
pub async fn signal(service: &str) {
    let interrupt = tokio::signal::ctrl_c();
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("{service}: can't listen for SIGTERM: {error}");
                let _ = interrupt.await;
                return;
            }
        };
    tokio::select! {
        _ = interrupt => {}
        _ = terminate.recv() => {}
    }
}
