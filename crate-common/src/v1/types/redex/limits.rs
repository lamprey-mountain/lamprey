// TODO: allow configuring max cpu time per time period
// TODO: allow bursts of mem/cpu/etc usage

use lamprey_macros::record;

/// execution limits
#[record]
#[derive(PartialEq, Eq)]
pub struct EvalLimits {
    /// maximum memory usage in bytes
    pub max_memory: u64,

    /// maximum cpu wall time usage in milliseconds
    pub max_cpu_wall: u64,

    /// maximum cpu process time usage in milliseconds
    pub max_cpu_process: u64,
    // pub max_stack_size_bytes: usize,
}

impl EvalLimits {
    /// extremely strict limits
    // for now, while im testing, i don't want people to be able to blow up my server
    pub fn strict() -> Self {
        Self {
            max_memory: 8 * 1024 * 1024, // 8 MiB
            max_cpu_wall: 5000,          // 5 seconds
            max_cpu_process: 1000,       // 1 second
        }
    }
}
