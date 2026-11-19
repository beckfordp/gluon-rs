// SPDX-License-Identifier: MIT

//! The consumer's librdkafka configuration (moved from user-service-rs's
//! events-probe, gluon TD-S5): at-least-once, as fs2-kafka's in
//! notification-service (`AutoOffsetReset.Earliest`, commit after handling).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use kafka_client::settings::{ConsumerSettings, GroupId, SettingsError, Topic};

fn settings() -> ConsumerSettings {
    ConsumerSettings {
        bootstrap: "127.0.0.1:9092".to_owned(),
        group_id: "notification-service-user-events".parse().unwrap(),
        topic: "user-events".parse().unwrap(),
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

#[test]
fn a_topic_is_what_kafka_allows() {
    let topic: Topic = "user-events.v1_dlq".parse().unwrap();
    assert_eq!(topic.as_str(), "user-events.v1_dlq");
    assert_eq!(topic.to_string(), "user-events.v1_dlq");
    assert_eq!("".parse::<Topic>(), Err(SettingsError::Empty("topic")));
    assert_eq!(
        "user events".parse::<Topic>(),
        Err(SettingsError::Illegal("topic", "user events".to_owned()))
    );
    assert_eq!(
        "a".repeat(250).parse::<Topic>(),
        Err(SettingsError::TooLong("topic", "a".repeat(250)))
    );
    assert!("a".repeat(249).parse::<Topic>().is_ok());
    assert_eq!(
        "..".parse::<Topic>(),
        Err(SettingsError::Reserved("..".to_owned()))
    );
    assert_eq!(
        Topic::try_from("orders".to_owned()).unwrap().as_str(),
        "orders"
    );
}

#[test]
fn a_group_is_any_printable_name_up_to_kafkas_limit() {
    let group: GroupId = "notification-service-user-events".parse().unwrap();
    assert_eq!(group.as_str(), "notification-service-user-events");
    assert_eq!(group.to_string(), "notification-service-user-events");
    assert!("probe:run/7".parse::<GroupId>().is_ok());
    assert_eq!("".parse::<GroupId>(), Err(SettingsError::Empty("group id")));
    assert_eq!(
        "a group".parse::<GroupId>(),
        Err(SettingsError::Unprintable("a group".to_owned()))
    );
    assert_eq!(
        "g".repeat(250).parse::<GroupId>(),
        Err(SettingsError::TooLong("group id", "g".repeat(250)))
    );
}

#[test]
fn a_bad_name_says_what_and_why() {
    assert_eq!(SettingsError::Empty("topic").to_string(), "topic is empty");
    assert_eq!(
        SettingsError::Illegal("topic", "a b".to_owned()).to_string(),
        "topic \"a b\" may hold only letters, digits, '.', '_' and '-'"
    );
}
