// TODO: allow configuring max cpu time per time period
// TODO: allow bursts of mem/cpu/etc usage

#[derive(Debug, Clone)]
pub struct ChannelLimits {
    pub runtime: RuntimeLimits,
    pub run: RunLimits,
}

#[derive(Debug, Clone)]
pub struct RuntimeLimits {
    pub max_memory_bytes: usize,
    pub max_stack_size_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct RunLimits {
    pub max_instructions: u64,
    // TODO: execution time, API limits, etc.
}

impl Default for ChannelLimits {
    fn default() -> Self {
        Self {
            runtime: RuntimeLimits {
                max_memory_bytes: 8 * 1024 * 1024,
                max_stack_size_bytes: 512 * 1024,
            },
            run: RunLimits {
                max_instructions: 100_000,
            },
        }
    }
}

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
