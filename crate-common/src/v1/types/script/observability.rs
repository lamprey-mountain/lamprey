#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{metadata::MessageMetadata, misc::Time, RunId, ScriptId};

/// a log entry from a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunLogEntry {
    pub id: u64,
    pub created_at: Time,
    pub level: RunLogLevel,

    /// where this log came from
    pub source: RunLogSource,

    /// arbitrary content for this log line
    pub content: String,

    /// arbitrary metadata associated with this log line
    pub attributes: MessageMetadata,
}

/// source information for a log entry
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunLogSource {
    pub script_id: ScriptId,
    pub run_id: RunId,
    pub trace_id: Option<u64>,

    /// target (like rust foo::bar::baz) (like otel InstrumentationScope)
    pub target: String,

    /// the start of the span in utf8 codepoints (rust `char`s)
    pub span_start: u64,

    /// the end of the span in utf8 codepoints (rust `char`s)
    pub span_end: u64,
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
