// SPDX-License-Identifier: MIT

//! The config pieces every service reads the same way (both Scala services'
//! `application.conf`): ports, `POSTGRES_*`, and every problem reported at
//! once. Pure: an environment map in, values and problems out. Each service
//! keeps its own `Config` and calls these.

use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasher;

/// Where the service's database is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresConfig {
    /// `POSTGRES_HOST`.
    pub host: String,
    /// `POSTGRES_PORT`.
    pub port: u16,
    /// `POSTGRES_DB`.
    pub database: String,
    /// `POSTGRES_USER`.
    pub user: String,
    /// `POSTGRES_PASSWORD`.
    pub password: String,
}

/// Every problem with the environment, so one fix-up round is enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// One line per invalid variable.
    pub problems: Vec<String>,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid configuration:")?;
        for problem in &self.problems {
            write!(f, "\n- {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

/// `name` as a port number (trimmed), `default` when unset; a value that isn't
/// one is added to `problems` and `default` returned.
pub fn port<S: BuildHasher>(
    env: &HashMap<String, String, S>,
    name: &str,
    default: u16,
    problems: &mut Vec<String>,
) -> u16 {
    match env.get(name) {
        None => default,
        Some(raw) => raw.trim().parse::<u16>().unwrap_or_else(|_| {
            problems.push(format!(
                "{name} must be a port number (0-65535), got {raw:?}"
            ));
            default
        }),
    }
}

/// `POSTGRES_*`: host `localhost`, port 5432, and database, user and password
/// all `service_name`, as both Scala services default them.
pub fn postgres<S: BuildHasher>(
    env: &HashMap<String, String, S>,
    service_name: &str,
    problems: &mut Vec<String>,
) -> PostgresConfig {
    let text = |name: &str, default: &str| env.get(name).map_or(default, String::as_str).to_owned();
    PostgresConfig {
        host: text("POSTGRES_HOST", "localhost"),
        port: port(env, "POSTGRES_PORT", 5432, problems),
        database: text("POSTGRES_DB", service_name),
        user: text("POSTGRES_USER", service_name),
        password: text("POSTGRES_PASSWORD", service_name),
    }
}
