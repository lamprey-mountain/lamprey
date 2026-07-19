//! controlling which search results are returned

use common::v2::types::{ChannelId, RoomId, UserId};
use tantivy::query::Query;

/// Trait for converting visibility constraints into Tantivy queries.
pub trait TantivyVisibility {
    /// Convert the visibility constraint into a Tantivy query.
    fn into_query(self) -> Box<dyn Query>;
}

/// visibility for a single channel
#[derive(Debug, Clone)]
pub struct ChannelVisibility {
    /// the id of the channel
    pub id: ChannelId,

    /// whether to include private threads
    ///
    /// should be set to true if the `ThreadsManage` permission is enabled
    pub can_view_private_threads: bool,
}

// /// visibility for a user
// #[derive(Debug, Clone)]
// pub struct UserVisibility {
//     pub rooms: Vec<RoomId>,
//     pub gdms: Vec<ChannelId>,
//     pub friends: Vec<UserId>,
//     pub blocks: Vec<UserId>, // and ignores?
// }

/// what messages to include in the search
#[derive(Debug, Clone)]
pub enum MessagesFilter {
    /// all messages
    Everything,

    /// only messages in these filtered channels
    Filtered(Vec<ChannelVisibility>),
}

/// what channels to include in the search
#[derive(Debug, Clone)]
pub enum ChannelsFilter {
    /// all channels
    Everything,

    /// only channels in these rooms or owned by these users
    Filtered {
        /// for dms/gdms
        user_ids: Vec<UserId>,

        /// for regular channels
        room_ids: Vec<RoomId>,
    },
}

/// what rooms to include in the search
#[derive(Debug, Clone)]
pub enum RoomsFilter {
    /// public rooms + these rooms
    Public(Vec<RoomId>),

    /// only public rooms
    PublicOnly,

    /// all rooms
    Everything,
}

/// what applications to include in the search
#[derive(Debug, Clone)]
pub enum ApplicationsFilter {
    /// all applications
    Everything,

    /// only public applications
    PublicOnly,

    /// only applications owned by this user
    Owner(UserId),

    /// public applications or applications owned by this user
    PublicOrOwner(UserId),
}

/// what media to include in the search
#[derive(Debug, Clone)]
pub enum MediaFilter {
    /// all media
    Everything,

    /// only media from these users
    ///
    /// eg. a user and all their bots
    Users(Vec<UserId>),
}

/// what media to include in the search
#[derive(Debug, Clone)]
pub enum UserFilter {
    /// all users
    Everything,

    /// only these users
    // NOTE: are these friends? mutual room members/gdms? what else?
    Users(Vec<UserId>),
}

#[derive(Debug, Clone)]
pub enum AuditLogFilter {
    /// all media
    Everything,

    /// only entries from this room
    Room(RoomId),
}

// TODO: impl TantivyFilter for all of the above (copy from crate-backend/src/services/search/util/visibility.rs)
