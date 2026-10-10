use lamprey_macros::record;

use crate::v1::types::{ChannelId, InviteCode, UserId, misc::Time};

#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum Aggregation {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[record(params)]
#[derive(PartialEq, Eq)]
pub struct AnalyticsParams {
    pub start: Option<Time>,
    pub end: Option<Time>,
    pub aggregate: Aggregation,

    /// limit between 1..1024, default to 10
    pub limit: Option<u16>,
}

/// the count of members in this room
#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsMembersCount {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members in this room.
    pub count: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsMembersJoin {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members who joined this room.
    pub count: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsMembersLeave {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members who left this room.
    pub count: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsUsagesCount {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of usages.
    pub count: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsUsagesAdd {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of additions.
    pub count: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsUsagesRemove {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of removals.
    pub count: u64,
}

#[record(params)]
#[derive(PartialEq, Eq)]
pub struct AnalyticsChannelParams {
    /// return only analytics for this channel, otherwise return data points for everything
    pub channel_id: Option<ChannelId>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsChannel {
    /// The bucket for this data point.
    pub bucket: Time,
    pub channel_id: ChannelId,
    pub message_count: u64,
    pub media_count: u64,
    pub media_size: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsOverview {
    /// The bucket for this data point.
    pub bucket: Time,

    /// number of messages sent
    pub message_count: u64,

    /// number of files sent
    pub media_count: u64,

    /// number of files sent
    pub media_size: u64,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct AnalyticsInvites {
    /// The bucket for this data point.
    pub bucket: Time,

    /// where this member came from
    pub origin: AnalyticsInvitesOrigin,

    /// number of times this invite was used
    pub uses: u64,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type")]
pub enum AnalyticsInvitesOrigin {
    /// user joined with this invite code
    Invite { code: InviteCode },

    /// this was a bot that was installed manually
    BotInstall,

    /// this is a puppet user and was added by a bridge
    Bridged {
        /// the bridge that owns this puppet
        bridge_id: UserId,
    },

    /// user joined directly
    PublicJoin,

    /// unknown or anonymized origin
    Other,
}
