// SPDX-License-Identifier: MIT

//! The config pieces every service reads the same way: ports, `POSTGRES_*`,
//! and every problem reported at once (both Scala services' `application.conf`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use service_basics::config::{ConfigError, PostgresConfig, port, postgres};

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn an_unset_port_is_its_default_and_a_set_one_is_trimmed() {
    let mut problems = Vec::new();
    assert_eq!(port(&env(&[]), "X_PORT", 8080, &mut problems), 8080);
    assert_eq!(
        port(&env(&[("X_PORT", " 9000 ")]), "X_PORT", 8080, &mut problems),
        9000
    );
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn a_bad_port_is_a_problem_naming_the_variable_and_value() {
    let mut problems = Vec::new();
    let bad = env(&[("A_PORT", "70000"), ("B_PORT", "http")]);
    assert_eq!(port(&bad, "A_PORT", 1, &mut problems), 1);
    assert_eq!(port(&bad, "B_PORT", 2, &mut problems), 2);
    assert_eq!(
        problems,
        [
            "A_PORT must be a port number (0-65535), got \"70000\"",
            "B_PORT must be a port number (0-65535), got \"http\"",
        ]
    );
}

#[test]
fn postgres_defaults_to_localhost_and_the_services_own_name() {
    let mut problems = Vec::new();
    assert_eq!(
        postgres(&env(&[]), "notification", &mut problems),
        PostgresConfig {
            host: "localhost".to_owned(),
            port: 5432,
            database: "notification".to_owned(),
            user: "notification".to_owned(),
            password: "notification".to_owned(),
        }
    );
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn postgres_reads_every_variable() {
    let mut problems = Vec::new();
    let set = env(&[
        ("POSTGRES_HOST", "db"),
        ("POSTGRES_PORT", "6543"),
        ("POSTGRES_DB", "d"),
        ("POSTGRES_USER", "u"),
        ("POSTGRES_PASSWORD", "p"),
    ]);
    assert_eq!(
        postgres(&set, "user", &mut problems),
        PostgresConfig {
            host: "db".to_owned(),
            port: 6543,
            database: "d".to_owned(),
            user: "u".to_owned(),
            password: "p".to_owned(),
        }
    );
}

#[test]
fn a_bad_postgres_port_joins_the_other_problems() {
    let mut problems = vec!["earlier".to_owned()];
    postgres(&env(&[("POSTGRES_PORT", "x")]), "user", &mut problems);
    assert_eq!(
        problems,
        [
            "earlier",
            "POSTGRES_PORT must be a port number (0-65535), got \"x\""
        ]
    );
}

#[test]
fn a_config_error_lists_one_problem_per_line() {
    let error = ConfigError {
        problems: vec!["one".to_owned(), "two".to_owned()],
    };
    assert_eq!(error.to_string(), "invalid configuration:\n- one\n- two");
}
