// SPDX-License-Identifier: MIT

//! librdkafka's client context: its statistics become the lag gauge
//! (purekafka `ConsumerLag`).

use kafka_event_consumer::lag::ConsumerMetrics;
use rdkafka::ClientContext;
use rdkafka::consumer::ConsumerContext;

/// The consumer's context: where librdkafka's statistics go.
pub struct LagContext {
    /// The lag gauge (and event age) of the consumer's group.
    pub metrics: ConsumerMetrics,
}

impl ClientContext for LagContext {
    fn stats_raw(&self, statistics: &[u8]) {
        self.metrics
            .record_lag(&String::from_utf8_lossy(statistics));
    }
}

impl ConsumerContext for LagContext {}
