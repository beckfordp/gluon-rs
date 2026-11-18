// SPDX-License-Identifier: MIT

//! The dead-letter producer is created without a broker (librdkafka connects
//! lazily); delivering needs one, so the contract harness exercises that.

use kafka_client::publisher::KafkaPublisher;

#[test]
fn a_producer_is_created_without_reaching_the_broker() {
    assert!(KafkaPublisher::new("127.0.0.1:9").is_ok());
}
