use lamprey_macros::record;

use crate::{
    v1::types::{
        Channel, Role, Room, RoomMember, Session, User, application::Application,
        preferences::PreferencesGlobal,
    },
    v2::types::{
        ConnectionId,
        sync::dispatch::{channel::DispatchChannel, room::DispatchRoom, user::DispatchUser},
    },
};

pub mod channel;
pub mod invite;
pub mod media;
pub mod room;
pub mod user;
pub mod webhook;

/// something happened
#[record]
#[serde(tag = "op")]
pub enum Dispatch {
    /// extra context
    ///
    /// this is sent after Ready and is generally needed for the client to function
    Ambient(Ambient),

    // TODO: require a subscription to a room to receive room dispatches, same with channel. only user dispatches should be sent to the client.
    #[serde(untagged)]
    Room(DispatchRoom),

    #[serde(untagged)]
    Channel(DispatchChannel),

    #[serde(untagged)]
    User(DispatchUser),
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

// TODO: decide what goes here? returning all rooms could be bad for large bots
// maybe only send room summaries, require subscribing to rooms to retrieve full ambient data and start receiving dispatches
#[record]
pub struct Ambient {
    /// the user's global preferences
    pub config: PreferencesGlobal,

    /// all rooms the user can see
    pub rooms: Vec<Room>,

    /// all roles in all rooms the user can see
    pub roles: Vec<Role>,

    /// all non-thread channels the user can see
    pub channels: Vec<Channel>,

    /// all active (ie. not archived) threads the user can see
    pub threads: Vec<Channel>,

    /// the user's room member object for each room the user is in
    pub room_members: Vec<RoomMember>,
    // NOTE: maybe i should include even more data
    // - friends/relationships (including friend requests)
    // - dms
    // - emoji
}
