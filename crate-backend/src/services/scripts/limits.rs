// TODO: allow configuring max cpu time per time period
// TODO: allow bursts of mem/cpu/etc usage

pub use common::v1::types::script::ChannelLimits;

pub struct RuntimeStats {
    pub instruction_count: u64,
    // ...
}

pub trait MetricsSink {
    fn measure(&self, memory_bytes: usize, total_instruction_count: u64);
}

pub struct MetricsCollector {
    pub samples: Vec<MetricsCollectorSample>,
}

pub struct MetricsCollectorSample {
    pub timestamp: std::time::Instant,
    pub memory_bytes: usize,
    pub instruction_count: u64,
}

impl MetricsSink for MetricsCollector {
    fn measure(&self, _memory_bytes: usize, _total_instruction_count: u64) {
        todo!()
    }
}

impl MetricsCollector {
    /// merge together older entries into 1 second chunks
    pub fn coalesce(&self) {
        todo!()
    }
}
