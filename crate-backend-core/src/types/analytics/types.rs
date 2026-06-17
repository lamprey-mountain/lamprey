use common::v1::types::{ChannelId, MediaId, RoomId, RoomMemberOrigin, UserId, util::Time};

use crate::types::analytics::{AbuseMetadata, ResourceAction};

#[derive(Debug, Clone)]
pub enum AnalyticsEvent {
    Distinct(AnalyticsEventDistinct),
    Aggregated(AnalyticsEventAggregated),
}

/// a single distict analytics event data point
#[derive(Debug, Clone)]
pub struct AnalyticsEventDistinct {
    pub inner: AnalyticsEventDistinctType,
    pub abuse: Option<AbuseMetadata>,
}

#[derive(Debug, Clone)]
pub enum AnalyticsEventDistinctType {
    /// user joined a voice channel
    VoiceJoin {
        channel_id: ChannelId,
        user_id: UserId,
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
        action: ResourceAction,
        room_id: RoomId,
    },

    /// a user was created, updated, or deleted
    User {
        action: ResourceAction,
    },

    /// a piece of media was created, updated, or deleted
    Media {
        action: ResourceAction,
        media_id: MediaId,
    },

    /// a piece of media was created or deleted
    ///
    /// When media is created or deleted, two analytics events are created: `Media` and `MediaSize`. Media is for media count aggregation, `MediaSize` is for media size aggregation.
    MediaSize {
        media_id: MediaId,

        /// only non zero during Create
        bytes_added: u64,

        /// only non zero during Delete
        bytes_removed: u64,
    },

    /// a channel was created, updated, or deleted
    Channel {
        action: ResourceAction,
        channel_id: ChannelId,
    },

    /// user was authenticated
    Auth {
        /// the user id that they tried to log into
        user_id: Option<UserId>,

        /// whether this auth was successful
        success: bool,
    },
}

/// an aggregated and anonymized set of analytics events
#[derive(Debug, Clone)]
pub struct AnalyticsEventAggregated {
    pub inner: AnalyticsEventAggregatedType,

    /// the start of this bucket's time
    pub time_start: Time,

    /// the end of this bucket's time
    pub time_end: Time,
}

/// some notes
///
/// - that Auth is removed, theres no good way to aggregate it
/// - media is aggregated in multiple ways
#[derive(Debug, Clone)]
pub enum AnalyticsEventAggregatedType {
    VoiceJoin {
        channel_id: ChannelId,
        count: u64,
    },

    VoiceLeave {
        channel_id: ChannelId,
        count: u64,
    },

    MemberJoin {
        /// the origin if it is known
        origin: Option<RoomMemberOrigin>,
        count: u64,
    },

    MemberLeave {
        count: u64,
    },

    Room {
        count_created: u64,
        count_deleted: u64,
    },

    User {
        count_created: u64,
        count_deleted: u64,
    },

    Channel {
        room_id: RoomId,
        count_created: u64,
        count_deleted: u64,
    },

    // NOTE: is there a better way to do media count/size by global/room/user variants?
    MediaCountGlobal {
        count_created: u64,
        count_deleted: u64,
    },

    MediaCountByRoom {
        room_id: RoomId,
        count_created: u64,
        count_deleted: u64,
    },

    MediaCountByUser {
        user_id: UserId,
        count_created: u64,
        count_deleted: u64,
    },

    MediaSizeGlobal {
        bytes_created: u64,
        bytes_deleted: u64,
    },

    MediaSizeByRoom {
        room_id: RoomId,
        bytes_created: u64,
        bytes_deleted: u64,
    },

    MediaSizeByUser {
        user_id: UserId,
        bytes_created: u64,
        bytes_deleted: u64,
    },

    /// message aggregation events
    ///
    /// these are too spammy to individually log, so they're only included as aggregated events
    // TODO: increment a counter in memory (nats?) and flush to tantivy every once in a while
    Message {
        channel_id: ChannelId,
        count_created: u64,
        count_deleted: u64,
    },
}
