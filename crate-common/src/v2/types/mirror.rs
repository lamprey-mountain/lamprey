use lamprey_macros::record;

use crate::{
    v1::types::{
        Channel, ChannelSeq, Message, MessageVersion,
        reaction::{Reaction, ReactionInfo},
    },
    v2::types::{MessageId, MessageVerId},
};

// TODO: impl and use
// NOTE: the v1 mirroring api returns a list of sync events, wheras this api returns an actual diff. unsure which one is better

// TODO: how do i handle users? they don't belong to channels/rooms
// maybe include user create/update/delete events for channel, room, and user mirrors
// room channels don't mirror users at all as they're handled at the room level

/// response for the channel mirror endpoint
///
/// contains a diff to apply to local state
#[record]
pub struct ChannelMirror {
    /// messages that were created
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_create: Vec<Message>,

    /// messages that were updated
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_update: Vec<Message>,

    /// messages that were deleted
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_delete: Vec<MessageId>,

    /// message versions that were created
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_version_create: Vec<MessageVersion>, // TODO: include message id

    /// message versions that were updated
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_version_update: Vec<MessageVersion>, // TODO: include message id

    /// message versions that were deleted
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_version_delete: Vec<MessageVerId>, // TODO: include message id

    /// reactions that were created
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reaction_create: Vec<Reaction>,

    /// reactions that were deleted
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reaction_delete: Vec<ReactionInfo>,

    // TODO: handle message remove, restore, update
    // TODO: handle reaction bulk delete
    // TODO: handle channel create, update, delete/remove
    // TODO: channel create, update, delete
    /// the latest sequence number included in this diff
    pub seq: ChannelSeq,

    /// not all events were returned. call this endpoint again with the new `seq`
    pub partial: bool,
}

// TODO: use this?
// #[record]
// pub struct Delta<C, U, D> {
//     /// resources that were created
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub create: Vec<T>,
//
//     /// resources that were updated
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub update: Vec<T>,
//
//     /// resources that were deleted
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub delete: Vec<D>,
// }

/// query params for the channel mirror endpoint
#[record(params)]
pub struct ChannelMirrorQuery {
    /// the sequence number to sync from (exclusive). use 0 to get all events.
    // TODO: set default = 0
    pub seq: ChannelSeq,

    // TODO: set min, max, default utoipa
    // TODO: set min, max validator
    #[serde(default = "default_limit")]
    pub limit: u16,
    // TODO: sync only a specific range of messages
    // maybe split channel mirror/message mirror?
}

/// response for the room mirror endpoint
///
/// contains a diff to apply to local state
#[record]
pub struct RoomMirror {
    /// channels that were created
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_create: Vec<Channel>,

    /// channels that were updated
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_update: Vec<Channel>,
    // TODO: add channel_delete...?
    // /// channels that were deleted
    // #[serde(skip_serializing_if = "Vec::is_empty")]
    // pub channel_delete: Vec<ChannelId>,
    // TODO: sync roles, members, emoji
}

pub const fn default_limit() -> u16 {
    100
}

// TODO: user sync: users, relationships, dm channels
