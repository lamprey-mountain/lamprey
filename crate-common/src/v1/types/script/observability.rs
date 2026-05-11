#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{metadata::MessageMetadata, misc::Time, ScriptId};

/// a log entry from a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunLogEntry {
    pub id: u64,
    pub created_at: Time,
    pub level: RunLogLevel,

    /// where this log line came from
    pub source: RunLogSource,

    /// arbitrary content for this log line
    pub content: String,

    /// arbitrary metadata associated with this log line
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "MessageMetadata::is_empty")
    )]
    pub attributes: MessageMetadata,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RunLogSource {
    /// this log line came from a user written script
    Script {
        /// the script this came from
        script_id: ScriptId,

        /// the trace this belongs to
        // TODO
        trace_id: Option<u64>, // TODO: newtype for trace id

        /// target (like rust foo::bar::baz) (like otel InstrumentationScope)
        target: String,

        /// the start of the span in utf8 codepoints (rust `char`s)
        span_start: u64,

        /// the end of the span in utf8 codepoints (rust `char`s)
        span_end: u64,
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
pub enum RunLogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

/// a trace span from a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunTrace {
    pub id: u64,
    pub created_at: Time,
    pub ended_at: Option<Time>,
    pub source: RunLogSource,
    pub label: String,

    /// arbitrary metadata associated with this trace
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "MessageMetadata::is_empty")
    )]
    pub attributes: MessageMetadata,
}

/// metrics collected from a single script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunMetrics {
    // counter: only ever increments
    // gauge: can go up or down
    // histogram: distribution (min/max/sum/count)
    //
    // builtin metrics: memory, cpu, gc
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(validator::Validate))]
pub struct RunMetricsQuery {
    // TODO: copy room analytics probaby
}
