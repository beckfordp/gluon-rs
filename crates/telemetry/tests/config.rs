// SPDX-License-Identifier: MIT

//! The OTEL variables, read once for every service (purerest
//! `TracingConfig.fromEnv`; gluon TECHNICAL-DEBT.md S-2): the endpoint trimmed,
//! blank as unset; the sampler ratio in [0, 1], 1 by default, a bad one
//! reported with the service's other problems.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use telemetry::config::{TracingConfig, tracing_from_env};

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn read(pairs: &[(&str, &str)]) -> (TracingConfig, Vec<String>) {
    let mut problems = Vec::new();
    let config = tracing_from_env(&env(pairs), &mut problems);
    (config, problems)
}

#[test]
fn unset_means_no_exporter_and_every_root_sampled() {
    assert_eq!(
        read(&[]),
        (
            TracingConfig {
                otlp_endpoint: None,
                sampling_ratio: 1.0
            },
            vec![]
        )
    );
}

#[test]
fn blank_is_unset() {
    let (config, problems) = read(&[
        ("OTEL_EXPORTER_OTLP_ENDPOINT", "  "),
        ("OTEL_TRACES_SAMPLER_ARG", " "),
    ]);
    assert_eq!(config.otlp_endpoint, None);
    assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn the_endpoint_is_trimmed_and_the_ratio_read() {
    let (config, problems) = read(&[
        ("OTEL_EXPORTER_OTLP_ENDPOINT", " http://127.0.0.1:4318 "),
        ("OTEL_TRACES_SAMPLER_ARG", "0.25"),
    ]);
    assert_eq!(
        config,
        TracingConfig {
            otlp_endpoint: Some("http://127.0.0.1:4318".to_owned()),
            sampling_ratio: 0.25
        }
    );
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn a_ratio_outside_0_to_1_or_not_a_number_is_a_problem() {
    for raw in ["2", "-0.1", "half"] {
        let (config, problems) = read(&[("OTEL_TRACES_SAMPLER_ARG", raw)]);
        assert_eq!(
            problems,
            [format!(
                "OTEL_TRACES_SAMPLER_ARG must be a number between 0 and 1, got {raw:?}"
            )]
        );
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    }
}

#[test]
fn problems_already_found_are_kept() {
    let mut problems = vec!["PORT must be a port number".to_owned()];
    tracing_from_env(&env(&[("OTEL_TRACES_SAMPLER_ARG", "x")]), &mut problems);
    assert_eq!(problems.len(), 2);
    assert_eq!(problems[0], "PORT must be a port number");
}
