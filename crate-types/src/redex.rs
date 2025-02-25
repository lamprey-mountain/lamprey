use serde::{Deserialize, Serialize};

use crate::{util::Time, MediaId, RedexId, UserId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// WARN: EXTREME work in progress. not going to happen anytime soon. maybe in the far future tho.

/// executable code that runs on le server
/// for automoderation and some special bot stuff
/// though, i'm a bit worried about the security
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Redex {
    pub id: RedexId,
    pub creator_id: UserId,

    /// the code that will be run
    #[serde(flatten)]
    pub code: RedexCode,

    /// input for the redex (so you don't have to reupload code to reconfigure)
    #[serde(flatten)]
    pub context: RedexContext,

    #[serde(flatten)]
    pub status: RedexStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "code_type")]
pub enum RedexCode {
    /// hopefully not too hard to implement
    WebAssembly {
        /// the media id where this code can be downloaded from
        code_media_id: MediaId,
    },

    /// to make it friendlier, maybe i could add a builtin scripting language
    Script {
        /// the media id where this code can be downloaded from
        code_media_id: MediaId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "code_type")]
pub enum RedexContext {
    /// small inline data
    Inline {
        /// the context
        context_data: Vec<u8>,
    },

    /// too big, was uploaded as media
    Media {
        /// the media id where this context can be downloaded from
        context_media_id: MediaId,
    },
}

/// everyone asks what is a redex nobody asks how is the redex
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "status")]
pub enum RedexStatus {
    /// the server is processing the code
    Installing,

    /// waiting for an old version to cleanup
    Waiting { since: Time },

    /// active and running
    Active { since: Time },

    /// finished running and exited
    Stopped { at: Time },

    /// encountered an error
    Killed { error: RedexError },
}

/// all the ways a redex can fail
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[non_exhaustive]
pub enum RedexError {
    /// took too long to run
    ExceededTime,

    /// took too much memory
    ExceededMemory,

    /// exceeded ratelimits (trying to do too much stuff)
    ExceededRatelimitOutbound,

    /// exceeded ratelimits (trying to subscribe to too many events)
    ExceededRatelimitInbound,

    /// went past some other kind of quota
    ExceededQuota,

    /// you can't do that (and forgot to handle this error)
    Unauthorized,

    /// the redex intentionally crashed fatally
    Panic,

    /// you tried send bad data through a syscall
    BadSyscall,

    /// there was an error while handling an error that tried to handle another error
    TripleFault,

    /// a new redex has been Waiting for too long, so this one is being forcefully shut down
    Upgrading,

    /// whoops looks like we don't support that
    Unsupported,
}

impl RedexError {
    /// all redexes will stop after too many errors (how many?), but some will immediately stop and not try to restart
    pub fn is_fatal(&self) -> bool {
        // panic is intentional
        // unsupported means that the redex is likely incompatible with this version altogether
        // upgrading shouldn't restart because a new version is trying to start first
        matches!(
            self,
            RedexError::Panic | RedexError::Unsupported | RedexError::Upgrading
        )
    }
}

/// a log entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexLog {
    /// the data in the log
    pub entry: String,

    /// the attributes associated with the log
    pub attrs: Vec<(String, String)>,

    /// the log level
    pub level: RedexLogLevel,

    /// timestamp
    pub ts: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,

    /// acts as a panic
    Fatal,
}

// TODO: figure out how to create wasm (wit?) bindings
// mod automod {
//     use super::RedexLogLevel;

//     struct Event;
//     // how to generate

//     /// stuff the system has to implement
//     trait System {
//         // observability
//         fn log(&self, level: RedexLogLevel, data: &str);
//         fn log_with(&self, level: RedexLogLevel, data: &str, attrs: &[(&str, &str)]);
//         fn metric_set(&self, counter: &str, val: u64);
//         fn metric_incr(&self, counter: &str, val: u64);
//         fn metric_count(&self, counter: &str);

//         // api
//         fn room_member_get(&self);
//         fn room_member_update(&self);
//         fn room_member_kick(&self);
//         fn invite_resolve(&self);
//         fn invite_delete(&self);
//         fn invite_room_create(&self);
//         fn invite_thread_create(&self);
//         fn room_edit(&self);
//         fn message_create(&self);
//         // etc...
//     }

//     trait Redex {
//         // receive something from the system
//         fn handle_event(&self, event: Event);
//     }
// }
