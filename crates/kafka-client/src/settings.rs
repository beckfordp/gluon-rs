// SPDX-License-Identifier: MIT

//! What a consumer is: its broker, group and topic, and how often librdkafka
//! reports the statistics the lag gauge is read from.

use std::time::Duration;

use rdkafka::ClientConfig;

/// One consumer's settings: one group on one topic, as each of
/// notification-service's consumers is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerSettings {
    /// The brokers (`KAFKA_BOOTSTRAP_SERVERS`).
    pub bootstrap: String,
    /// The consumer group, e.g. `notification-service-user-events`.
    pub group_id: String,
    /// The topic it reads.
    pub topic: String,
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
            .set("group.id", &self.group_id)
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
