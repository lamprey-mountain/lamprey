#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{
    v1::types::{
        AuditLogEntry, AuditLogEntryId, Channel, ChannelId, MediaId, Message, MessageId, Room,
        RoomId, RoomMember, ThreadMember, User, UserId,
    },
    v2::types::media::Media,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSearch {
    /// the ids of the matched messages
    pub results: Vec<MessageId>,

    /// all relevant messages (eg. messages that a result replied to)
    pub messages: Vec<Message>,

    /// the authors of the messages
    pub users: Vec<User>,

    /// threads the messages are in
    pub threads: Vec<Channel>,

    /// room members objects for each author, if they exist
    pub room_members: Vec<RoomMember>,

    /// relevant thread member objects
    ///
    /// - one for each (message author, thread) tuple
    /// - one for each thread the requesting user is a member of
    pub thread_members: Vec<ThreadMember>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelSearch {
    /// the ids of the matched channels
    pub results: Vec<ChannelId>,

    /// the channels
    pub channels: Vec<Channel>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserSearch {
    /// the ids of the matched users
    pub results: Vec<UserId>,

    /// the users
    pub users: Vec<User>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomSearch {
    /// the ids of the matched rooms
    pub results: Vec<RoomId>,

    /// the rooms
    pub rooms: Vec<Room>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaSearch {
    /// the ids of the matched media
    pub results: Vec<MediaId>,

    /// the media
    pub media: Vec<Media>,

    /// the media creators/uploaders
    pub user: Vec<User>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogSearch {
    /// the ids of the matched audit log entries
    pub results: Vec<AuditLogEntryId>,

    /// the audit log entries
    pub entries: Vec<AuditLogEntry>,

    // TODO: copy AuditLogPaginationResponse here
    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
