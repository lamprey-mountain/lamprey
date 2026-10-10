//! utility for traversing and collecting references to other resources

use crate::v1::types::{ChannelId, RoomId, UserId};

/// visitor to handle resource references
#[allow(unused_variables)]
pub trait Visitor {
    /// visit a user
    fn visit_user(&mut self, user_id: UserId) {}

    /// visit a channel
    ///
    /// this includes threads
    fn visit_channel(&mut self, channel_id: ChannelId) {}

    /// visit a room
    fn visit_room(&mut self, room_id: RoomId) {}

    /// visit a room member
    ///
    /// this also calls visit_user
    fn visit_room_member(&mut self, user_id: UserId, room_id: RoomId) {}

    /// visit a channel (thread) member
    ///
    /// this also calls visit_user
    fn visit_channel_member(&mut self, user_id: UserId, channel_id: ChannelId) {}

    // TODO: visit media, application, webhook, tag, etc
}

/// this data references other resources
pub trait Context {
    /// visit all references
    fn visit<V: Visitor>(&self, visitor: &mut V);
}
