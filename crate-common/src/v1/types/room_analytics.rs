#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{misc::Time, ChannelId, InviteCode, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Aggregation {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct RoomAnalyticsParams {
    pub start: Option<Time>,
    pub end: Option<Time>,
    pub aggregate: Aggregation,

    /// limit between 1..1024, default to 10
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsMembersCount {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members in this room.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsMembersJoin {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members who joined this room.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsMembersLeave {
    /// The bucket for this data point.
    pub bucket: Time,

    /// Total number of members who left this room.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct RoomAnalyticsChannelParams {
    /// return only analytics for this channel, otherwise return data points for everything
    pub channel_id: Option<ChannelId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsChannel {
    /// The bucket for this data point.
    pub bucket: Time,
    pub channel_id: ChannelId,
    pub message_count: u64,
    pub media_count: u64,
    pub media_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsOverview {
    /// The bucket for this data point.
    pub bucket: Time,

    /// number of messages sent
    pub message_count: u64,

    /// number of files sent
    pub media_count: u64,

    /// number of files sent
    pub media_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomAnalyticsInvites {
    /// The bucket for this data point.
    pub bucket: Time,

    /// where this member came from
    pub origin: RoomAnalyticsInvitesOrigin,

    /// number of times this invite was used
    pub uses: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
pub enum RoomAnalyticsInvitesOrigin {
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
