#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// the execution limits configured for this channel
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelLimits {
    pub runtime: RuntimeLimits,
    pub run: RunLimits,
}

/// the execution limits configured across all runs for all scripts in this channel
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RuntimeLimits {
    pub max_memory_bytes: usize,
    pub max_stack_size_bytes: usize,
}

/// the execution limits configured for each run
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunLimits {
    pub max_instructions: u64,
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
