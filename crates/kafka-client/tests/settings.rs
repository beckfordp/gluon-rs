// SPDX-License-Identifier: MIT

//! The consumer's librdkafka configuration (moved from user-service-rs's
//! events-probe, gluon TD-S5): at-least-once, as fs2-kafka's in
//! notification-service (`AutoOffsetReset.Earliest`, commit after handling).

use std::time::Duration;

use kafka_client::settings::ConsumerSettings;

fn settings() -> ConsumerSettings {
    ConsumerSettings {
        bootstrap: "127.0.0.1:9092".to_owned(),
        group_id: "notification-service-user-events".to_owned(),
        topic: "user-events".to_owned(),
        lag_interval: Duration::from_secs(5),
    }
}

#[test]
fn the_group_reads_from_the_earliest_offset_and_commits_itself() {
    let config = settings().client_config();
    let get = |key| config.get(key).map(ToOwned::to_owned);
    assert_eq!(get("bootstrap.servers").as_deref(), Some("127.0.0.1:9092"));
    assert_eq!(
        get("group.id").as_deref(),
        Some("notification-service-user-events")
    );
    assert_eq!(get("enable.auto.commit").as_deref(), Some("false"));
    assert_eq!(get("auto.offset.reset").as_deref(), Some("earliest"));
    assert_eq!(get("allow.auto.create.topics").as_deref(), Some("true"));
}

#[test]
fn librdkafka_reports_statistics_at_the_lag_interval() {
    let config = ConsumerSettings {
        lag_interval: Duration::from_millis(1500),
        ..settings()
    }
    .client_config();
    assert_eq!(config.get("statistics.interval.ms"), Some("1500"));
}
