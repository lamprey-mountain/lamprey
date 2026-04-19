#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{misc::Time, RunId, ScriptId};

/// a script execution run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Run {
    pub id: RunId,
    pub script_id: ScriptId,
    pub created_at: Time,
    pub stopped_at: Option<Time>,
    pub status: RunStatus,
}

/// status of a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RunStatus {
    Running,
    Success,

    /// error while running the script
    RuntimeFailure,

    /// error with types, syntax, etc
    PreflightFailure,
}

/// request to start a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunCreate {
    /// start in the background
    ///
    /// returns 202 accepted instead of blocking until it can return 200 ok
    #[cfg_attr(feature = "serde", serde(rename = "async"))]
    pub run_async: bool,

    /// whether only one instance should be running at a time
    ///
    /// will stop other runs of this script if true
    pub exclusive: bool,
}
