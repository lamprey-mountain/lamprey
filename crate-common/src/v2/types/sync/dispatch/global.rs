use crate::{
    v1::types::{
        Channel, InteractionId, Role, Room, RoomMember, UserId,
        interactions::{Interaction, InteractionErrorCode},
        preferences::PreferencesGlobal,
    },
    v2::types::sync::dispatch::{
        channel::{ChannelCreate, ChannelUpdate},
        room::{RoomCreate, RoomDelete, RoomUpdate},
    },
};
use lamprey_macros::record;

/// something happened
///
/// always sent to the client
#[record]
#[serde(tag = "type")]
pub enum DispatchGlobal {
    Ambient(Ambient),
    RoomCreate(RoomCreate),
    RoomUpdate(RoomUpdate),
    RoomDelete(RoomDelete),
    ChannelCreate(ChannelCreate),
    ChannelUpdate(ChannelUpdate),
    InteractionCreate(InteractionCreate),
    InteractionSuccess(InteractionSuccess),
    InteractionFailure(InteractionFailure),
}

// TODO: decide what goes here? returning all rooms could be bad for large bots
// maybe only send room summaries, require subscribing to rooms to retrieve full ambient data and start receiving dispatches
/// extra context
///
/// this is sent after Ready and is generally needed for the client to function
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

/// an interaction was created
///
/// sent to the the user who created this and the target application
#[record]
pub struct InteractionCreate {
    pub interaction: Box<Interaction>,
    pub user_id: UserId,
    pub nonce: Option<String>,
}

/// an interaction succeeded
#[record]
pub struct InteractionSuccess {
    pub user_id: UserId,
    pub interaction_id: InteractionId,
    pub nonce: Option<String>,
}

/// an interaction failed
#[record]
pub struct InteractionFailure {
    pub user_id: UserId,
    pub interaction_id: InteractionId,
    pub nonce: Option<String>,
    pub error_code: InteractionErrorCode,
}
