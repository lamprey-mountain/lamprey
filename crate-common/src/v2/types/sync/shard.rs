#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::{
    v1::types::{misc::Time, SyncCompression, SyncVersion},
    v2::types::{sync::filter::DispatchFilter, ChannelId, ShardId, SyncId},
};

use super::SyncEncoding;

/// how events should be/are being received
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Transport {
    /// using a websocket
    Websocket,

    /// using a webtransport connection
    // /// open or reuse a webtransport connection
    Webtransport {
        // /// if id already is used, multiplex over existing webtransport
        // id: u16,
        // stream_id: u16,

        // /// ID of the established WT session
        // session_id: u32,
        // /// Native QUIC stream ID to use (if applicable)
        // stream_id: Option<u16>,
    },

    /// send to a webhook
    ///
    /// the webhook must respond with a 2xx status code (generally 202 accepted) within 3 seconds
    ///
    /// webhooks are somewhat limited in that they can't use any `SyncCommand`s
    Webhook {
        /// the url to send events to
        url: Url,

        /// secret key for signing events
        // TODO: probably will be ed25519 or something, reuse federation header?
        // TODO: better types?
        secret_key: String,

        /// the current status of this webhook
        status: SyncWebhookStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncWebhookStatus {
    /// server is validating webhook
    Pending,

    /// webhook is ok
    Alive,

    /// webhook keeps timing out requests
    Timeout,

    /// webhook doesnt handle signing properly
    Invalid,

    /// webhook manually disabled
    Disabled,
}

impl SyncWebhookStatus {
    /// whether this status represents an error state
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Timeout | Self::Invalid)
    }

    /// whether this status is in the alive state
    pub fn is_alive(&self) -> bool {
        *self == Self::Alive
    }
}

/// a logical session/connection to the service
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Syncer {
    /// the unique identifier of a sync connection
    // TODO: use this instead of ConnectionId?
    pub id: SyncId,

    /// the main transport, if it exists
    pub transport: Option<Transport>,

    /// extra shards
    pub shard: Vec<Shard>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncerCreate {
    // TODO
}

/// a stream of events
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Shard {
    /// the unique identifier of this shard
    pub id: ShardId,

    /// override the transport for this shard
    pub transport: Option<Transport>,

    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ShardKind,

    /// whether this shard is currently connected to
    pub active: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ShardCreate {
    // TODO
}

/// the kind of events that are received
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ShardKind {
    /// master event bus
    Dispatch {
        /// the numeric index of this shard
        #[cfg_attr(feature = "serde", serde(default))]
        shard: u16,

        /// the total number of shards to split events across
        #[cfg_attr(feature = "serde", serde(default))]
        total_shards: u16,

        #[cfg_attr(feature = "serde", serde(default))]
        filter: DispatchFilter,
    },

    /// voice signalling
    Voice,

    /// document editing and presence
    Document { channel_id: ChannelId },

    /// only interaction events
    // NOTE: similar to ShardKind::Dispatch with a strict filter?
    Interactions,
}

/// limits and configuration for a sync session
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct SyncLimits {
    /// the recommended number of shards to use when connecting
    pub shards_recommended: u64,

    /// how many more shards can be opened
    pub shards_remaining: u64,

    /// the time at which `shards_remaining` resets
    pub reset_after: Time,

    /// the maximum number of shards to start simultaneously
    pub max_concurrency: u64,
}

/// query parameters when establishing a websocket (or webtransport) sync connection
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct WebsocketSyncParams {
    pub version: SyncVersion,

    pub compression: Option<SyncCompression>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub encoding: SyncEncoding,
}
