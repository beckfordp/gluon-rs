// SPDX-License-Identifier: MIT

//! Config helpers every service shares (`PureConfig`'s readings, as purerest
//! services get them): HOCON durations, and the OTEL variables (purerest
//! `TracingConfig.fromEnv`) in the shape of gluon-rs `service-basics`'
//! readers: an environment map in, problems collected with the service's.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::time::Duration;

/// Where spans go, and how many.
#[derive(Debug, Clone, PartialEq)]
pub struct TracingConfig {
    /// `OTEL_EXPORTER_OTLP_ENDPOINT`; `None` when unset or blank.
    pub otlp_endpoint: Option<String>,
    /// `OTEL_TRACES_SAMPLER_ARG`, in [0, 1]; 1.0 when unset or blank.
    pub sampling_ratio: f64,
}

/// Reads the OTEL variables; a sampler ratio that isn't a number in [0, 1] is
/// added to `problems` (and 1.0 used), so a service reports every bad
/// variable at once.
pub fn tracing_from_env<S: BuildHasher>(
    env: &HashMap<String, String, S>,
    problems: &mut Vec<String>,
) -> TracingConfig {
    let otlp_endpoint = env
        .get("OTEL_EXPORTER_OTLP_ENDPOINT")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let sampling_ratio = match env.get("OTEL_TRACES_SAMPLER_ARG").map(|v| v.trim()) {
        None | Some("") => 1.0,
        Some(raw) => match raw.parse::<f64>() {
            Ok(ratio) if (0.0..=1.0).contains(&ratio) => ratio,
            _ => {
                problems.push(format!(
                    "OTEL_TRACES_SAMPLER_ARG must be a number between 0 and 1, got {raw:?}"
                ));
                1.0
            }
        },
    };
    TracingConfig {
        otlp_endpoint,
        sampling_ratio,
    }
}

/// Parses a HOCON duration as Typesafe Config does: a number and an optional
/// unit (`ns`, `us`, `ms`, `s`, `m`, `h`, `d` or their long forms); no unit
/// means milliseconds.
///
/// # Errors
/// A message saying what was wrong with `text`.
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    let trimmed = text.trim();
    let split = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(split);
    let amount: u64 = number
        .parse()
        .map_err(|_| format!("{text:?} is not a duration (e.g. 1h, 30d, 500ms)"))?;
    let unit_nanos: u64 = match unit.trim() {
        "ns" | "nano" | "nanos" | "nanosecond" | "nanoseconds" => 1,
        "us" | "micro" | "micros" | "microsecond" | "microseconds" => 1_000,
        "" | "ms" | "milli" | "millis" | "millisecond" | "milliseconds" => 1_000_000,
        "s" | "second" | "seconds" => 1_000_000_000,
        "m" | "minute" | "minutes" => 60_000_000_000,
        "h" | "hour" | "hours" => 3_600_000_000_000,
        "d" | "day" | "days" => 86_400_000_000_000,
        other => return Err(format!("unknown duration unit {other:?} in {text:?}")),
    };
    amount
        .checked_mul(unit_nanos)
        .map(Duration::from_nanos)
        .ok_or_else(|| format!("{text:?} is too long"))
}
