#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{EvalId, RedexId};

// TODO: use this in routes

/// an error that occured while running a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexError {
    /// the id of the run where this error occured
    pub run_id: EvalId,

    /// where this error occured
    pub location: RedexErrorLocation,

    /// human readable message
    pub message: String,

    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: RedexErrorType,
}

/// an error that occured while running a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexErrorType {
    /// runtime type error
    RuntimeType,

    /// runtime syntax error
    RuntimeSyntax,

    RuntimeReference,
    RuntimeRange,

    // TODO: replace these two with more exact errors?
    RuntimeInternal,
    RuntimeGeneric,

    /// took too long to run
    ExceededTime,

    /// took too much memory
    ExceededMemory,
}

/// location metadata for this error
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexErrorLocation {
    /// the id of the redex that the error occured in
    pub redex_id: RedexId,

    /// the stack trace for this error
    pub stack_trace: String,

    /// target (like rust foo::bar::baz) (like otel InstrumentationScope)
    pub target: String,

    /// the line that this error occured on
    pub line: u64,

    /// the column that this error occured on
    pub column: u64,
}

impl std::fmt::Display for RedexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RedexError({}) in {}", self.message, self.run_id)
    }
}
