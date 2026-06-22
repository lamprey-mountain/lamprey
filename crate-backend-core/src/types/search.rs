use std::str::FromStr;

use common::{v1::types::ChannelId, v2::types::RoomId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// a request to reindex stuff on the server
#[derive(Debug, Clone)]
pub struct Reindex {
    /// reindex only these types of documents
    pub doctypes: Vec<Doctype>,

    /// reindex only documents in these rooms
    ///
    /// includes the room iself. empty vec means no filter.
    pub room_ids: Vec<RoomId>,

    /// reindex only documents in these channels
    ///
    /// includes the channel iself. empty vec means no filter.
    pub channel_ids: Vec<ChannelId>,
}

/// a queue that needs to be reindexed
#[derive(Debug, Clone)]
pub struct SearchReindexQueue {
    pub target: SearchReindexQueueTarget,
    pub last_item_id: Option<Uuid>,
}

/// the target of a search reindex queue
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SearchReindexQueueTarget {
    /// messages in a channel
    Messages(ChannelId),

    /// channels on the server
    Channels,

    /// rooms on the server
    Rooms,

    /// users on the server
    Users,

    /// media on the server
    Media,

    /// audit log entries in a room
    AuditLogEntries(RoomId),
}

/// the type of a tantivy document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Doctype {
    /// document represents a message
    Message,

    /// document represents a channel
    Channel,

    /// document represents a room
    Room,

    /// document represents an user
    User,

    /// document represents a piece of media
    Media,

    /// document represents an audit log entry
    AuditLogEntry,

    /// document represents an analytics event
    AnalyticsEvent,

    /// document represents a change to a document
    DocumentChange,
    // TODO: more searching
    // Document, // branch_id, template, draft, published, view_count(?)(sorting)
    // Tag, // usage_count(sorting)
    // Application, // public(admin only), usage_count(sorting)
    // CalendarEvent, // location, starts_at, ends_at
    // RoomTemplate, // usage_count(sorting)
    // Emoji, // animated, usage_count(sorting)
    // Broadcasts, // member_count(sorting)
}

impl Doctype {
    /// get this document type as a string
    pub fn as_str(&self) -> &str {
        match self {
            Doctype::Message => "Message",
            Doctype::Channel => "Channel",
            Doctype::Room => "Room",
            Doctype::User => "User",
            Doctype::Media => "Media",
            Doctype::AuditLogEntry => "AuditLogEntry",
            Doctype::AnalyticsEvent => "AnalyticsEvent",
            Doctype::DocumentChange => "DocumentChange",
        }
    }
}

impl AsRef<str> for Doctype {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Doctype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Doctype {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Message" => Ok(Doctype::Message),
            "Channel" => Ok(Doctype::Channel),
            "Room" => Ok(Doctype::Room),
            "User" => Ok(Doctype::User),
            "Media" => Ok(Doctype::Media),
            "AuditLogEntry" => Ok(Doctype::AuditLogEntry),
            "AnalyticsEvent" => Ok(Doctype::AnalyticsEvent),
            "DocumentChange" => Ok(Doctype::DocumentChange),
            _ => Err(()),
        }
    }
}

impl Reindex {
    /// check if this reindex request is empty (no filters)
    pub fn is_empty(&self) -> bool {
        self.doctypes.is_empty() && self.room_ids.is_empty() && self.channel_ids.is_empty()
    }

    /// create a new [`Reindex`] operation to reindex everything
    pub fn everything() -> Self {
        Self {
            doctypes: vec![],
            room_ids: vec![],
            channel_ids: vec![],
        }
    }
}
