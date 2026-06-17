#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{RedexId, metadata::Metadata, misc::Time};

/// a log entry from an eval
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EvalLogEntry {
    pub id: u64,
    pub created_at: Time,
    pub level: EvalLogLevel,

    /// where this log line came from
    pub source: EvalLogSource,

    /// arbitrary content for this log line
    pub content: String,

    /// arbitrary metadata associated with this log line
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Metadata::is_empty"))]
    pub attributes: Metadata,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EvalLogSource {
    /// this log line came from a user written redex
    Redex {
        /// the redex this came from
        redex_id: RedexId,

        /// the trace this belongs to
        // TODO
        trace_id: Option<u64>, // TODO: newtype for trace id

        /// target (like rust foo::bar::baz) (like otel InstrumentationScope)
        target: Option<String>,

        line: Option<u64>,
        column: Option<u64>,
    },

    /// log came from an internal/builtin module
    Builtin {
        /// target (like rust foo::bar::baz) (like otel InstrumentationScope)
        target: String,
    },

    /// this log line came from the runtime itself
    Runtime,
}

/// log level for a run log entry
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EvalLogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

// TODO: metrics, traces
// /// a trace span from a script run
// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct RunTrace {
//     pub id: u64,
//     pub created_at: Time,
//     pub ended_at: Option<Time>,
//     pub source: EvalLogSource,
//     pub label: String,

//     /// arbitrary metadata associated with this trace
//     #[cfg_attr(
//         feature = "serde",
//         serde(skip_serializing_if = "MessageMetadata::is_empty")
//     )]
//     pub attributes: MessageMetadata,
// }

// /// metrics collected from a single script run
// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct RunMetrics {
//     // counter: only ever increments
//     // gauge: can go up or down
//     // histogram: distribution (min/max/sum/count)
//     //
//     // builtin metrics: memory, cpu, gc
// }

// #[derive(Debug, Default, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
// #[cfg_attr(feature = "validator", derive(validator::Validate))]
// pub struct RunMetricsQuery {
//     // TODO: copy room analytics probaby
// }
