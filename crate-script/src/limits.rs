// TODO: allow configuring max cpu time per time period
// TODO: allow bursts of mem/cpu/etc usage

// pub use common::v1::types::script::ChannelLimits;

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Limits {
    /// maximum memory usage in bytes
    pub max_memory: usize,

    /// maximum cpu wall time usage
    pub max_cpu_wall: Duration,

    /// maximum cpu process time usage
    pub max_cpu_process: Duration,
    // pub max_stack_size_bytes: usize,
}

impl Limits {
    /// extremely strict limits
    // for now, while im testing, i don't want people to be able to blow up my server
    pub fn strict() -> Self {
        Self {
            max_memory: 8 * 1024 * 1024,
            max_cpu_wall: Duration::from_secs(5),
            max_cpu_process: Duration::from_secs(1),
        }
    }
}

// pub struct RuntimeStats {
//     pub instruction_count: u64,
//     // ...
// }

// pub trait MetricsSink {
//     fn measure(&self, memory_bytes: usize, total_instruction_count: u64);
// }

// pub struct MetricsCollector {
//     pub samples: Vec<MetricsCollectorSample>,
// }

// pub struct MetricsCollectorSample {
//     pub timestamp: std::time::Instant,
//     pub memory_bytes: usize,
//     pub instruction_count: u64,
// }

// impl MetricsSink for MetricsCollector {
//     fn measure(&self, _memory_bytes: usize, _total_instruction_count: u64) {
//         todo!()
//     }
// }

// impl MetricsCollector {
//     /// merge together older entries into 1 second chunks
//     pub fn coalesce(&self) {
//         todo!()
//     }
// }
