use std::net::IpAddr;

use common::v1::types::{ChannelId, MediaId, RoomId, RoomMemberOrigin, SessionId, UserId};

#[derive(Debug, Clone)]
pub enum AnalyticsEventPayload {
    /// user joined a voice channel
    VoiceJoin {
        channel_id: ChannelId,
        user_id: Option<UserId>,
    },

    /// user left a voice channel
    VoiceLeave {
        channel_id: ChannelId,
        user_id: Option<UserId>,

        /// whether the user cleanly left or not (eg. internet issues)
        clean: bool,
    },

    MemberJoin {
        user_id: Option<UserId>,
        // including invite origin in aggregated data still be identifiable if a single (or very few) users used the invite
        // "singling out via small population bucket"
        // fix: const MIN_BUCKET_SIZE: u64 = 20;
        // keep the bucket in tantivy, but replace the bucket with "unknown origin" in the api?
        origin: RoomMemberOrigin,
    },

    MemberLeave {
        user_id: Option<UserId>,
    },

    /// a room was created, updated, or deleted
    Room {
        action: AnalyticsResourceAction,
        room_id: Option<RoomId>,
        user_id: Option<UserId>,
    },

    /// a user was created, updated, or deleted
    User {
        action: AnalyticsResourceAction,
        user_id: Option<UserId>,
    },

    /// a piece of media was created, updated, or deleted
    Media {
        action: AnalyticsResourceAction,
        media_id: Option<MediaId>,
        user_id: Option<UserId>,
    },

    /// a channel was created, updated, or deleted
    Channel {
        action: AnalyticsResourceAction,
        channel_id: Option<ChannelId>,
        user_id: Option<UserId>,
    },

    // message events are too spammy to log individual events, only include as aggregated events
    // increment a counter in memory (nats?) and flush to tantivy every once in a while
    Message {
        channel_id: ChannelId,
    },

    Auth {
        /// the user id that they tried to log into
        user_id: Option<UserId>,

        /// whether this auth was successful
        success: bool,
    },
}

#[derive(Debug, Clone)]
pub enum AnalyticsResourceAction {
    Create,
    Update,
    Delete,
}

/// metadata for abuse monitoring
pub struct AbuseMetadata {
    /// the ip address of the request that caused this event
    pub ip_addr: IpAddr,

    /// the user agent of the request that caused this event
    pub user_agent: String,

    /// the id of the session that caused this request
    ///
    /// may be None for unauthenticated requests
    pub session_id: Option<SessionId>,
    // TODO: include extra metadata later:
    // request_method, request_path
    // ja3_fingerprint
    // asn/country code
}
