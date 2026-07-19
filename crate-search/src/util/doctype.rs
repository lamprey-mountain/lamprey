use std::str::FromStr;

/// the type of a tantivy document
// TODO: derive serde?
// TODO: use strum instead
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
