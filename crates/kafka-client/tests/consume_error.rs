// SPDX-License-Identifier: MIT

//! Why a consumer stream fails (gluon TD-64.1): each `ConsumeError` reads as the
//! `String` it replaced did, so the supervisor's restart line is unchanged, and
//! keeps its cause as the source.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::error::Error;

use kafka_client::KafkaError;
use kafka_client::consumer::ConsumeError;
use kafka_event_consumer::dead_letter::DeadLetterError;
use rdkafka::types::RDKafkaErrorCode;

fn kafka(code: RDKafkaErrorCode) -> KafkaError {
    KafkaError::ConsumerCommit(code)
}

#[test]
fn a_failed_commit_says_so_and_keeps_its_cause() {
    let error = ConsumeError::Commit(kafka(RDKafkaErrorCode::BrokerTransportFailure));
    let cause = kafka(RDKafkaErrorCode::BrokerTransportFailure).to_string();
    assert_eq!(error.to_string(), format!("commit failed: {cause}"));
    assert!(error.source().is_some());
}

#[test]
fn broker_and_subscribe_failures_read_as_rdkafkas() {
    let cause = kafka(RDKafkaErrorCode::AllBrokersDown);
    let text = cause.to_string();
    assert_eq!(ConsumeError::AllBrokersDown(cause).to_string(), text);
    let cause = kafka(RDKafkaErrorCode::UnknownTopic);
    let text = cause.to_string();
    assert_eq!(ConsumeError::Subscribe(cause).to_string(), text);
}

#[test]
fn a_failed_dead_letter_is_the_streams_error() {
    let error = ConsumeError::from(DeadLetterError::BreakerOpen);
    assert_eq!(error.to_string(), "Circuit breaker is open");
    assert!(matches!(
        error,
        ConsumeError::DeadLetter(DeadLetterError::BreakerOpen)
    ));
}
