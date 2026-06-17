use std::collections::HashMap;

use bytes::Bytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{EvalId, MessageSync, RedexId, RedexVerId, UserId, misc::Time};

/// a redex being run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Eval {
    pub id: EvalId,
    pub redex_id: RedexId,
    pub redex_version_id: RedexVerId,
    pub created_at: Time,
    pub stopped_at: Option<Time>,
    pub status: EvalStatus,
    pub input: EvalInputSummary,
}

/// request to start a redex run via trigger
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EvalCreateManual {
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

/// status of an eval
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EvalStatus {
    /// the eval is being created and started
    ///
    /// valid transitions: Active, Sleeping, Exited, Crashed
    Creating,

    /// the eval is active
    ///
    /// valid transitions: Sleeping, Exited, Crashed
    Active,

    /// the eval is pausing or paused and stored on disk
    ///
    /// valid transitions: Waking
    Sleeping,

    /// the eval is starting up
    ///
    /// valid transitions: Active, Exited, Crashed
    Waking,

    /// the eval has exited cleanly
    ///
    /// valid transitions: (none)
    Exited,

    /// the eval is borked (preflight failure: syntax, types, compile time, etc)
    ///
    /// valid transitions: (none)
    Borked,

    /// the eval has crashed (runtime failure)
    ///
    /// valid transitions: (none)
    Crashed,

    /// the eval was stopped manually
    ///
    /// valid transitions: (none)
    Stopped,
}

// /// why an eval was stopped
// pub enum EvalStopReason {
//     /// eval was explicitly killed by a user
//     Killed { user_id: UserId, reason: String },
//     // ExtractionFailure(ErrorMessage),
//     // RuntimeFailure(ErrorMessage),
//     // Exited(Message),
// }

/// valid input for a script
#[derive(Debug, Clone)]
pub enum EvalInput {
    /// only extract
    Extraction,

    /// manual trigger
    Manual { id: String, user_id: UserId },

    /// http request
    Http { request: http::Request<Bytes> },

    /// api event (MessageSync)
    Event { event: Box<MessageSync> },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EvalInputSummary {
    Extraction,

    /// manual trigger
    Manual {
        id: String,
        user_id: UserId,
    },

    /// http request
    Http {
        request: HttpRequestSummary,
    },

    /// api event
    Event {
        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        event: Box<MessageSync>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct HttpRequestSummary {
    pub request_method: String,
    pub request_url: String,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
    pub response_status: u16,
}

impl From<EvalInput> for EvalInputSummary {
    fn from(value: EvalInput) -> Self {
        match value {
            EvalInput::Extraction => EvalInputSummary::Extraction,
            EvalInput::Manual { id, user_id } => EvalInputSummary::Manual { id, user_id },
            EvalInput::Http { request } => EvalInputSummary::Http {
                request: HttpRequestSummary {
                    request_method: request.method().to_string(),
                    request_url: request.uri().to_string(),
                    request_headers: request
                        .headers()
                        .iter()
                        .filter_map(|(name, value)| {
                            // TODO: warn on header drop?
                            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
                        })
                        .collect(),
                    // TODO: populate later
                    response_headers: Default::default(),
                    response_status: Default::default(),
                },
            },
            EvalInput::Event { event } => EvalInputSummary::Event { event },
        }
    }
}

impl EvalStatus {
    /// Transition to the given status if the transition is valid.
    ///
    /// Returns `true` if the transition is allowed, `false` otherwise.
    ///
    /// Terminal states (`Exited`, `Borked`, `Crashed`, `Stopped`) cannot transition to any other state.
    pub fn transition_to(&self, next: EvalStatus) -> bool {
        match (self, next) {
            // Creating can go to any active or terminal state
            (
                EvalStatus::Creating,
                EvalStatus::Active
                | EvalStatus::Sleeping
                | EvalStatus::Exited
                | EvalStatus::Crashed
                | EvalStatus::Stopped,
            ) => true,

            // Active can go to sleeping, exited, crashed, or stopped
            (
                EvalStatus::Active,
                EvalStatus::Sleeping
                | EvalStatus::Exited
                | EvalStatus::Crashed
                | EvalStatus::Stopped,
            ) => true,

            // Sleeping can only wake up
            (EvalStatus::Sleeping, EvalStatus::Waking) => true,

            // Waking can go to active, exited, crashed, or stopped
            (
                EvalStatus::Waking,
                EvalStatus::Active | EvalStatus::Exited | EvalStatus::Crashed | EvalStatus::Stopped,
            ) => true,

            // Terminal states cannot transition anywhere
            (
                EvalStatus::Exited | EvalStatus::Borked | EvalStatus::Crashed | EvalStatus::Stopped,
                _,
            ) => false,

            // Anything else is invalid
            _ => false,
        }
    }
}
