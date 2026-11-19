// SPDX-License-Identifier: MIT

//! What a consumer is: its broker, group and topic, and how often librdkafka
//! reports the statistics the lag gauge is read from.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use rdkafka::ClientConfig;

/// One consumer's settings: one group on one topic, as each of
/// notification-service's consumers is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerSettings {
    /// The brokers (`KAFKA_BOOTSTRAP_SERVERS`).
    pub bootstrap: String,
    /// The consumer group, e.g. `notification-service-user-events`.
    pub group_id: GroupId,
    /// The topic it reads.
    pub topic: Topic,
    /// How often librdkafka emits statistics, hence the lag gauge.
    pub lag_interval: Duration,
}

impl ConsumerSettings {
    /// librdkafka's configuration: at least once (no auto-commit; the loop
    /// commits after handling), from the earliest offset for a new group, as
    /// fs2-kafka's `AutoOffsetReset.Earliest` in notification-service.
    #[must_use]
    pub fn client_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", &self.bootstrap)
            .set("group.id", self.group_id.as_str())
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("allow.auto.create.topics", "true")
            .set(
                "statistics.interval.ms",
                self.lag_interval.as_millis().to_string(),
            );
        config
    }
}

/// Kafka's longest topic name (and the longest group id it's sensible to give).
const MAX_NAME: usize = 249;

/// A name that isn't a valid topic or group id: a config error at startup.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// Nothing given.
    #[error("{0} is empty")]
    Empty(&'static str),
    /// Longer than Kafka allows.
    #[error("{0} {1:?} is longer than 249 characters")]
    TooLong(&'static str, String),
    /// A topic character Kafka refuses.
    #[error("{0} {1:?} may hold only letters, digits, '.', '_' and '-'")]
    Illegal(&'static str, String),
    /// A group id with a space or control character.
    #[error("group id {0:?} may not hold spaces or control characters")]
    Unprintable(String),
    /// `.` or `..`, which Kafka reserves.
    #[error("topic {0:?} is reserved")]
    Reserved(String),
}

fn bounded(what: &'static str, name: &str) -> Result<(), SettingsError> {
    if name.is_empty() {
        return Err(SettingsError::Empty(what));
    }
    if name.chars().count() > MAX_NAME {
        return Err(SettingsError::TooLong(what, name.to_owned()));
    }
    Ok(())
}

/// A topic name, as Kafka allows it: 1 to 249 of letters, digits, `.`, `_`, `-`;
/// not `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(String);

impl Topic {
    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Topic {
    type Err = SettingsError;

    fn from_str(name: &str) -> Result<Self, SettingsError> {
        bounded("topic", name)?;
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(SettingsError::Illegal("topic", name.to_owned()));
        }
        if name == "." || name == ".." {
            return Err(SettingsError::Reserved(name.to_owned()));
        }
        Ok(Self(name.to_owned()))
    }
}

impl TryFrom<String> for Topic {
    type Error = SettingsError;

    fn try_from(name: String) -> Result<Self, SettingsError> {
        name.parse()
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A consumer group id: 1 to 249 characters, no spaces or control characters
/// (Kafka allows more; nothing here needs it).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupId(String);

impl GroupId {
    /// The id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for GroupId {
    type Err = SettingsError;

    fn from_str(id: &str) -> Result<Self, SettingsError> {
        bounded("group id", id)?;
        if id.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(SettingsError::Unprintable(id.to_owned()));
        }
        Ok(Self(id.to_owned()))
    }
}

impl TryFrom<String> for GroupId {
    type Error = SettingsError;

    fn try_from(id: String) -> Result<Self, SettingsError> {
        id.parse()
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
