use common::{
    v1::types::MessageSync,
    v2::types::{ChannelId, RoomId},
};

// TODO: move to common
/// get the room id for a message sync event
pub fn sync_room_id(sync: &MessageSync) -> Option<RoomId> {
    match sync {
        MessageSync::RoomCreate { room } => Some(room.id),
        MessageSync::ChannelCreate { channel } => channel.room_id,
        // TODO: handle more events
        _ => None,
    }
}

// TODO: move to common
pub fn sync_channel_id(sync: &MessageSync) -> Option<ChannelId> {
    match sync {
        MessageSync::ChannelCreate { channel } => Some(channel.id),
        // TODO: handle more events
        _ => None,
    }
}

// NOTE: maybe move this to common?
/// why a room isn't available
#[derive(Debug, Clone)]
pub enum UnavailableReason {
    /// the room could not be found
    NotFound,

    /// the room is deleted
    Deleted,

    /// the room is quarantined
    Quarantined,

    // /// the federated server the room is on is offline
    // FederationOffline,
    // FederationTimeout,
    // // etc..
    /// some other mysterious failure reason
    // TODO: remove
    Other,
    // /// too many events were received and the room actor is backlogged
    // Backlogged,
}

impl UnavailableReason {
    /// whether `.ready()` should always fail when this reason is encountered
    ///
    /// otherwise, ready may continue waiting for the room to become available
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::NotFound | Self::Deleted | Self::Quarantined)
    }
}
