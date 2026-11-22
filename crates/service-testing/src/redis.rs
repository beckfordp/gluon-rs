// SPDX-License-Identifier: MIT

//! A Redis server for one test, from the local `redis-server` (gluon all-Rust
//! Stage A P0.3): no persistence, its own port, stopped and restarted on that
//! port to try a service with Redis down.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::free_port;

/// `redis-server`: `REDIS_SERVER`, else the first on the PATH.
fn redis_server() -> PathBuf {
    if let Ok(explicit) = std::env::var("REDIS_SERVER") {
        return PathBuf::from(explicit);
    }
    let path = std::env::var("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join("redis-server"))
        .find(|candidate| candidate.is_file())
        .expect("redis-server on the PATH, or REDIS_SERVER")
}

/// A Redis server for one test, killed when dropped.
pub struct Redis {
    bin: PathBuf,
    child: Mutex<Option<Child>>,
    /// Its TCP port on 127.0.0.1.
    pub port: u16,
}

impl Redis {
    /// A fresh server, answering once this returns.
    #[must_use]
    pub fn start() -> Self {
        let redis = Self {
            bin: redis_server(),
            child: Mutex::new(None),
            port: free_port(),
        };
        redis.start_again();
        redis
    }

    /// Its URL, as a service's `REDIS_URL`.
    #[must_use]
    pub fn url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.port)
    }

    /// Starts it on its port again after [`Redis::stop`] (empty: no persistence).
    pub fn start_again(&self) {
        let child = Command::new(&self.bin)
            .args(["--port", &self.port.to_string(), "--bind", "127.0.0.1"])
            .args(["--save", "", "--appendonly", "no", "--daemonize", "no"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        *self.child.lock().unwrap() = Some(child);
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(("127.0.0.1", self.port)).is_err() {
            assert!(
                Instant::now() < deadline,
                "redis-server not listening within 10 s"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Stops it: nothing listens on its port until [`Redis::start_again`].
    pub fn stop(&self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Redis {
    fn drop(&mut self) {
        self.stop();
    }
}
