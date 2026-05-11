// TODO: allow configuring max cpu time per time period
// TODO: allow bursts of mem/cpu/etc usage

struct RuntimeLimits {
    max_runtime_memory: u64,
    max_context_stack_size: u64,
    max_context_instructions: u64,
    // instructions is per-context but memory is per-runtime. maybe i should create more runtimes?
}

struct RuntimeStats {
    instruction_count: u64,
    // ...
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_runtime_memory: Default::default(),
            max_context_stack_size: Default::default(),
            max_context_instructions: Default::default(),
        }
    }
}

impl RuntimeLimits {
    /// returns whether these stats exceed the configured limits
    pub fn exceeds_limit(&self, stats: &RuntimeStats) -> bool {
        todo!()
    }
}

trait MetricsSink {
    fn measure(&self, memory_bytes: usize, total_instruction_count: u64);
}

struct MetricsCollector {
    samples: Vec<MetricsCollectorSample>,
}

struct MetricsCollectorSample {
    timestamp: std::time::Instant,
    memory_bytes: usize,
    instruction_count: u64,
}

impl MetricsSink for MetricsCollector {
    fn measure(&self, memory_bytes: usize, total_instruction_count: u64) {
        todo!()
    }
}

impl MetricsCollector {
    /// merge together older entries into 1 second chunks
    pub fn coalesce(&self) {
        todo!()
    }
}
