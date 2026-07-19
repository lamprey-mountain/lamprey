pub mod globals;
pub mod services;

pub(crate) mod prelude {
    pub use crate::globals::{Globals, GlobalsOwned};
    pub use crate::services::Services;
    pub use bytes::Bytes;
    pub use lamprey_backend_core::prelude::*;
    pub use std::sync::Arc;
    pub type CoreResult<T, E> = ::core::result::Result<T, E>;
    pub use futures_util::StreamExt;
}

// TEMP: compatability
pub use lamprey_backend_core::prelude::*;

// TEMP: compatability
mod types {
    // Re-export types from the postgres data crate
    pub use lamprey_backend_data_postgres::*;

    // Re-export common types
    pub use common::v1::types::channel::*;
    pub use common::v1::types::invite::*;
    pub use common::v1::types::message::*;
    pub use common::v1::types::pagination::*;
    pub use common::v1::types::permission::*;
    pub use common::v1::types::role::*;
    pub use common::v1::types::room::*;
    pub use common::v1::types::room_member::*;
    pub use common::v1::types::session::*;
    pub use common::v1::types::sync::*;
    pub use common::v1::types::user::*;
    pub use common::v1::types::{emoji, notifications, reaction};

    pub use common::v1::types::misc::SessionIdReq;
    pub use common::v1::types::misc::UserIdReq;
    pub use common::v2::types::ApplicationId;
    pub use common::v2::types::MediaId;
    pub use common::v2::types::SERVER_ROOM_ID;
    pub use lamprey_backend_core::types::permission::PermissionBits;
}

// TEMP: compatability
mod consts {
    //! global constants

    // TODO: make most of these configurable per server
    // TODO: move the rest of these to crate-common, as they are protocol-level constraints

    /// the maximum number of roles per room. clients should be able to fetch everything in one request.
    pub const MAX_ROLE_COUNT: u32 = 1024;

    /// the maximum number of active channels per room. clients should be able to fetch everything in one request.
    pub const MAX_CHANNEL_COUNT: u32 = 1024;

    /// the maximum number of permission overwrites per channel
    pub const MAX_PERMISSION_OVERWRITES: u32 = 64;

    /// the maximum number of unique reaction emoji per message
    pub const MAX_UNIQUE_REACTIONS: u32 = 20;

    /// the maximum number of custom emoji per room. clients should be able to fetch everything in one request.
    // TODO: remove?
    pub const MAX_CUSTOM_EMOJI: u32 = 1024;

    /// the maximum number of pinned messages per channel. clients should be able to fetch everything in one request.
    pub const MAX_PINNED_MESSAGES: u32 = 1024;

    /// the maximum number of role members to add to a thread when a role is mentioned.
    pub const MAX_ROLE_MENTION_MEMBERS_ADD: u32 = 50;

    /// the maximum number of members to allow in group dm.
    pub const MAX_GDM_MEMBERS: u32 = 16;

    /// the maximum number of webhooks per channel
    pub const MAX_CHANNEL_WEBHOOKS: u32 = 16;

    /// the maximum number of rooms a user can be in.
    pub const MAX_ROOM_JOINS: u32 = 128;

    /// the maximum number of rooms to keep loaded in memory.
    pub const MAX_LOADED_ROOMS: u64 = 4096;

    /// how many days to retain audit log entries
    pub const RETENTION_AUDIT_LOG: u32 = 90;

    /// how many days to retain room analytics entries
    pub const RETENTION_ROOM_ANALYTICS: u32 = 180;

    /// how long to retain calls without any users for, in seconds (for Broadcast channels)
    pub const EMPTY_CALL_TIMEOUT: u64 = 300;

    /// how long to retain an inactive room in memory, in seconds.
    pub const IDLE_TIMEOUT_ROOM: u64 = 900;

    /// how long to retain an inactive member list in memory, in seconds.
    pub const IDLE_TIMEOUT_MEMBER_LIST: u64 = 900;

    /// the maximum number of messages to process in a single actor tick before yielding.
    // NOTE: should i reduce this? is there any penalty to yielding more often?
    pub const ROOM_ACTOR_MESSAGE_BUDGET: usize = 50;

    /// the maximum number of public connections a user can have
    ///
    /// ie. connections with not ConnectionVisibility::Private
    pub const MAX_PUBLIC_CONNECTIONS: usize = 32;

    /// the maximum number of active branches a document can have
    // TODO: make unlimited, then remove?
    pub const MAX_DOCUMENT_BRANCHES: usize = 64;

    /// the maximum number of loaded branches/editing contexts a document can have
    pub const MAX_LOADED_DOCUMENT_BRANCHES: usize = 32;

    // TODO: pinning documents to sidebar in ui
    // /// the maximum number of pinned documents per wiki a user can have
    // pub const MAX_DOCUMENT_PINS: usize = 32;

    /// the maximum file size of a script
    // TODO: raise this, wasm is pretty big
    pub const MAX_SCRIPT_FILE_SIZE: u64 = 64 * 1024; // 64 kb for now
}

// TEMP: compatability
mod error {
    pub use lamprey_backend_core::error::*;
}

// FIXME: ServerStateInner doesn't exist here
// FIXME: ServerState doesn't exist here
// FIXME: routes doesn't exist here
// FIXME: Auth4 doesn't exist here
