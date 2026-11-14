// SPDX-License-Identifier: MIT

//! A PostgreSQL cluster for one test, from the local binaries.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::free_port;

/// The PostgreSQL binaries: `CONTRACT_PG_BIN`, else the directory of `pg_ctl` on the PATH.
fn pg_bin() -> PathBuf {
    if let Ok(explicit) = std::env::var("CONTRACT_PG_BIN") {
        return PathBuf::from(explicit);
    }
    let path = std::env::var("PATH").unwrap_or_default();
    let pg_ctl = std::env::split_paths(&path)
        .map(|dir| dir.join("pg_ctl"))
        .find(|candidate| candidate.is_file())
        .expect("pg_ctl on the PATH, or CONTRACT_PG_BIN");
    pg_ctl
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A PostgreSQL cluster for one test: the `user` superuser with password
/// `user` over TCP, database `user`, as gluon-local's user-postgres.
pub struct Postgres {
    bin: PathBuf,
    data: tempfile::TempDir,
    socket: tempfile::TempDir,
    /// Its TCP port on 127.0.0.1.
    pub port: u16,
}

impl Postgres {
    /// A fresh cluster (`initdb`, start, `createdb user`).
    #[must_use]
    pub fn start() -> Self {
        let bin = pg_bin();
        let data = tempfile::tempdir().unwrap();
        let socket = tempfile::Builder::new()
            .prefix("rs-pg-")
            .tempdir_in("/tmp")
            .unwrap();
        let password = data.path().join("password");
        std::fs::write(&password, "user\n").unwrap();
        run(
            &bin.join("initdb"),
            &[
                "-D",
                data.path().join("db").to_str().unwrap(),
                "-U",
                "user",
                &format!("--pwfile={}", password.display()),
                "--auth-local=trust",
                "--auth-host=scram-sha-256",
                "--locale=C.UTF-8",
            ],
        );
        let postgres = Self {
            bin,
            data,
            socket,
            port: free_port(),
        };
        postgres.start_again();
        run(
            &postgres.bin.join("createdb"),
            &[
                "-h",
                postgres.socket_dir(),
                "-p",
                &postgres.port.to_string(),
                "-U",
                "user",
                "user",
            ],
        );
        postgres
    }

    fn socket_dir(&self) -> &str {
        self.socket.path().to_str().unwrap()
    }

    /// Stops it (fast shutdown); [`Postgres::start_again`] brings it back with its data.
    pub fn stop(&self) {
        run(
            &self.bin.join("pg_ctl"),
            &[
                "stop",
                "-D",
                self.data.path().join("db").to_str().unwrap(),
                "-m",
                "fast",
                "-w",
            ],
        );
    }

    /// Freezes the postmaster and its children (`kill -STOP`), as the
    /// capture's container pause: connections stay open, nothing answers.
    pub fn pause(&self) {
        assert!(self.signal_all("-STOP"));
    }

    /// Undoes [`Postgres::pause`].
    pub fn resume(&self) {
        assert!(self.signal_all("-CONT"));
    }

    /// Sends `signal` to the postmaster and its children; false if it isn't running.
    fn signal_all(&self, signal: &str) -> bool {
        let Ok(pid_file) = std::fs::read_to_string(self.data.path().join("db/postmaster.pid"))
        else {
            return false;
        };
        let postmaster = pid_file
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let children = Command::new("pgrep")
            .args(["-P", &postmaster])
            .output()
            .unwrap();
        let mut pids = vec![postmaster];
        pids.extend(
            String::from_utf8(children.stdout)
                .unwrap()
                .split_whitespace()
                .map(str::to_owned),
        );
        Command::new("kill")
            .arg(signal)
            .args(&pids)
            .status()
            .is_ok_and(|status| status.success())
    }

    /// Starts it on its port, with logical WAL (for CDC tests).
    pub fn start_again(&self) {
        let options = format!(
            "-c listen_addresses=127.0.0.1 -c port={} -c unix_socket_directories={} -c wal_level=logical",
            self.port,
            self.socket_dir()
        );
        run(
            &self.bin.join("pg_ctl"),
            &[
                "start",
                "-D",
                self.data.path().join("db").to_str().unwrap(),
                "-l",
                self.data.path().join("log").to_str().unwrap(),
                "-w",
                "-o",
                &options,
            ],
        );
    }

    /// The service's `POSTGRES_*` variables for this cluster.
    #[must_use]
    pub fn env(&self) -> Vec<(&'static str, String)> {
        vec![
            ("POSTGRES_HOST", "127.0.0.1".to_owned()),
            ("POSTGRES_PORT", self.port.to_string()),
            ("POSTGRES_DB", "user".to_owned()),
            ("POSTGRES_USER", "user".to_owned()),
            ("POSTGRES_PASSWORD", "user".to_owned()),
        ]
    }

    /// One SQL statement through psql; rows as `|`-separated lines (often run
    /// only for its effect, so not `must_use`).
    #[allow(clippy::must_use_candidate)]
    pub fn query(&self, sql: &str) -> String {
        let output = Command::new(self.bin.join("psql"))
            .args([
                "-X",
                "-tA",
                "-h",
                self.socket_dir(),
                "-p",
                &self.port.to_string(),
            ])
            .args(["-U", "user", "-d", "user", "-c", sql])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// `pg_dump --schema-only --no-owner -t table`, as the capture's `db/schema.sql`.
    #[must_use]
    pub fn pg_dump(&self, table: &str) -> String {
        let output = Command::new(self.bin.join("pg_dump"))
            .args(["-s", "--no-owner", "-t", table, "-h", self.socket_dir()])
            .args(["-p", &self.port.to_string(), "-U", "user", "user"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

impl Drop for Postgres {
    fn drop(&mut self) {
        self.signal_all("-CONT");
        let _ = Command::new(self.bin.join("pg_ctl"))
            .args([
                "stop",
                "-D",
                self.data.path().join("db").to_str().unwrap(),
                "-m",
                "immediate",
            ])
            .output();
    }
}

fn run(program: &Path, args: &[&str]) {
    let output = Command::new(program).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{} {args:?}: {}",
        program.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}
