use bytes::Bytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{misc::Time, MessageSync, RunId, ScriptId};

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
    pub input: RunInputSummary,
}

/// request to start a script run via trigger
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RunCreateTrigger {
    /// start in the background
    ///
    /// returns 202 accepted instead of blocking until it can return 200 ok
    #[cfg_attr(feature = "serde", serde(rename = "async"))]
    pub run_async: bool,

    /// whether only one instance should be running at a time
    ///
    /// will stop other runs of this script if true
    pub exclusive: bool,

    /// the id of the input that triggered this run
    pub trigger_id: String,
}

/// status of a script run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RunStatus {
    /// the run is being created and started
    ///
    /// valid transitions: Active, Sleeping, Exited, Crashed
    Creating,

    /// the run is active
    ///
    /// valid transitions: Sleeping, Exited, Crashed
    Active,

    /// the run is pausing or paused and stored on disk
    ///
    /// valid transitions: Waking
    Sleeping,

    /// the run is starting up
    ///
    /// valid transitions: Active, Exited, Crashed
    Waking,

    /// the run has exited cleanly
    ///
    /// valid transitions: (none)
    Exited,

    /// the run is borked (preflight failure: syntax, types, compile time, etc)
    ///
    /// valid transitions: (none)
    Borked,

    /// the run has crashed (runtime failure)
    ///
    /// valid transitions: (none)
    Crashed,

    /// the run was stopped manually
    ///
    /// valid transitions: (none)
    Stopped,
}

// pub enum RunStopReason {
//     Killed(UserId, Reason),
//     ExtractionFailure(ErrorMessage),
//     RuntimeFailure(ErrorMessage),
//     Exited(Message),
// }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RunInputSummary {
    Extraction,

    /// manual trigger
    Trigger {
        id: String,
    },

    /// http request
    Http {
        request: HttpRequestSummary,
    },

    /// api event
    Event {
        event: Box<MessageSync>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct HttpRequestSummary {
    pub method: String,
    pub url: String,
    // TODO: headers
}

/// valid input for a script
#[derive(Debug, Clone)]
pub enum RunInput {
    /// only extract
    Extraction,

    /// manual trigger
    Trigger { id: String },

    /// http request
    Http { request: http::Request<Bytes> },

    /// api event (MessageSync)
    Event { event: Box<MessageSync> },
}

impl From<RunInput> for RunInputSummary {
    fn from(value: RunInput) -> Self {
        match value {
            RunInput::Extraction => RunInputSummary::Extraction,
            RunInput::Trigger { id } => RunInputSummary::Trigger { id },
            RunInput::Http { request } => RunInputSummary::Http {
                request: HttpRequestSummary {
                    method: request.method().to_string(),
                    url: request.uri().to_string(),
                },
            },
            RunInput::Event { event } => RunInputSummary::Event { event },
        }
    }
}

impl RunStatus {
    /// Transition to the given status if the transition is valid.
    ///
    /// Returns `true` if the transition is allowed, `false` otherwise.
    ///
    /// Terminal states (`Exited`, `Borked`, `Crashed`, `Stopped`) cannot transition to any other state.
    pub fn transition_to(&self, next: RunStatus) -> bool {
        match (self, next) {
            // Creating can go to any active or terminal state
            (
                RunStatus::Creating,
                RunStatus::Active
                | RunStatus::Sleeping
                | RunStatus::Exited
                | RunStatus::Crashed
                | RunStatus::Stopped,
            ) => true,

            // Active can go to sleeping, exited, crashed, or stopped
            (
                RunStatus::Active,
                RunStatus::Sleeping | RunStatus::Exited | RunStatus::Crashed | RunStatus::Stopped,
            ) => true,

            // Sleeping can only wake up
            (RunStatus::Sleeping, RunStatus::Waking) => true,

            // Waking can go to active, exited, crashed, or stopped
            (
                RunStatus::Waking,
                RunStatus::Active | RunStatus::Exited | RunStatus::Crashed | RunStatus::Stopped,
            ) => true,

            // Terminal states cannot transition anywhere
            (
                RunStatus::Exited | RunStatus::Borked | RunStatus::Crashed | RunStatus::Stopped,
                _,
            ) => false,

            // Anything else is invalid
            _ => false,
        }
    }
}
