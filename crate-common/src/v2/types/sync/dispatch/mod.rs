//! Dispatch events related to the sync protocol
//!
//! Dispatches are the basic system for keeping state in sync between the client
//! and the server.

use lamprey_macros::record;

use crate::{
    v1::types::{Session, User, application::Application},
    v2::types::{
        ConnectionId,
        sync::dispatch::{
            channel::DispatchChannel, client::DispatchClient, global::DispatchGlobal,
            room::DispatchRoom, user::DispatchUser,
        },
    },
};

pub mod channel;
pub mod client;
pub mod global;
// pub mod invite;
// pub mod media;
pub mod room;
pub mod user;
// pub mod webhook;

/// something happened
// NOTE: probably not necessary to have this?
// NOTE: if this is just for permission checks, i could probably make it take 'a references
#[record]
#[serde(untagged)]
pub enum Dispatch {
    Global(DispatchGlobal),
    Room(DispatchRoom),
    Channel(DispatchChannel),
    User(DispatchUser),
    Client(DispatchClient),
    // TODO: other dispatches, e2ee dispatch, voice state

    // #[cfg(feature = "feat_e2ee")]
    // EncryptionDispatch {
    //     /// who to send this dispatch to
    //     user_id: UserId,
    //     payload: E2EEDispatch,
    // },

    // // TODO: redesign this type
    // /// a voice state was updated
    // VoiceState {
    //     /// the id of the user who's voice state was updated
    //     user_id: UserId,
    //     state: Option<Box<VoiceState>>,

    //     // HACK: make it possible to use this for auth checks
    //     #[cfg_attr(feature = "serde", serde(skip))]
    //     old_state: Option<Box<VoiceState>>,
    // },
}

// NOTE: is this a dispatch event?
/// identify handshake completed
///
/// contains basic information about the session and client
#[record]
pub struct Ready {
    /// current user, null if session is unauthorized
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,

    /// the associated application
    ///
    /// - if an application is connecting on behalf of a user, this is the application that is connecting.
    /// - if a bot is connecting, this is the application the bot belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<Box<Application>>,

    /// current session
    pub session: Box<Session>,

    /// connection id
    pub conn: ConnectionId,
    // /// the syncer object
    // syncer: Box<Syncer>,
    // /// the id of this shard, if this is a sharded connection
    // shard_id: Option<ShardId>,
}

impl From<DispatchGlobal> for Dispatch {
    fn from(d: DispatchGlobal) -> Self {
        Self::Global(d)
    }
}

impl From<DispatchRoom> for Dispatch {
    fn from(d: DispatchRoom) -> Self {
        Self::Room(d)
    }
}

impl From<DispatchChannel> for Dispatch {
    fn from(d: DispatchChannel) -> Self {
        Self::Channel(d)
    }
}

impl From<DispatchUser> for Dispatch {
    fn from(d: DispatchUser) -> Self {
        Self::User(d)
    }
}

impl From<DispatchClient> for Dispatch {
    fn from(d: DispatchClient) -> Self {
        Self::Client(d)
    }
}
