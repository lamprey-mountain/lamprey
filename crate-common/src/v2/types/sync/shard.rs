use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::{SyncCompression, SyncVersion, misc::Time},
    v2::types::{ChannelId, ShardId, SyncId, sync::filter::DispatchFilter},
};

use super::SyncEncoding;

/// how events should be/are being received
#[record]
#[serde(tag = "type")]
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

#[record]
#[derive(PartialEq, Eq)]
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
#[record]
pub struct Syncer {
    /// the unique identifier of a sync connection
    // TODO: use this instead of ConnectionId?
    pub id: SyncId,

    /// the main transport, if it exists
    pub transport: Option<Transport>,

    /// extra shards
    pub shard: Vec<Shard>,
}

#[record]
pub struct SyncerCreate {
    // TODO
}

/// a stream of events
#[record]
pub struct Shard {
    /// the unique identifier of this shard
    pub id: ShardId,

    /// override the transport for this shard
    pub transport: Option<Transport>,

    #[serde(rename = "type")]
    pub ty: ShardKind,

    /// whether this shard is currently connected to
    pub active: bool,
}

#[record]
pub struct ShardCreate {
    // TODO
}

/// the kind of events that are received
#[record]
#[serde(tag = "type")]
pub enum ShardKind {
    /// master event bus
    Dispatch {
        /// the numeric index of this shard
        #[serde(default)]
        shard: u16,

        /// the total number of shards to split events across
        #[serde(default)]
        total_shards: u16,

        #[serde(default)]
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
#[record]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
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
#[record]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct WebsocketSyncParams {
    pub version: SyncVersion,

    pub compression: Option<SyncCompression>,

    #[serde(default)]
    pub encoding: SyncEncoding,
}
