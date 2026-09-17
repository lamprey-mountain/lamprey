use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::{SyncCompression, SyncVersion, misc::Time},
    v2::types::{ChannelId, ShardId, SyncId, sync::filter::DispatchFilter},
};

use super::SyncEncoding;

// TODO: design this (and the rest api) better
// cf https://dev.twitch.tv/docs/eventsub/handling-conduit-events/
// discord doesnt make you configure shards up front. maybe this would be useful?

// SyncId: a single logical session
// ShardId: a concrete transport
// ConnectionId: remove?

/// a logical session/connection to the service
#[record]
pub struct Syncer {
    /// the unique identifier of a sync connection
    pub id: SyncId,

    /// shards for this syncer
    pub shard: Vec<Shard>,
}

/// create a new syncer
#[record]
pub struct SyncerCreate {
    /// the initial shards to create
    // TODO: validate initial_shards is not empty
    // TODO: default to vec![ShardCreate { ... }]
    // default/basic syncer has a single shard with every event
    #[serde(default)]
    pub initial_shards: Vec<ShardCreate>,
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

/// a stream of events
#[record]
pub struct Shard {
    /// the unique identifier of this shard
    pub id: ShardId,

    /// the transport that should be used for this shard
    pub transport: Transport,

    /// whether this shard is currently connected to
    pub active: bool,
}

#[record]
pub struct ShardCreate {
    // TODO
}

/// the kind of events that are received
// TODO: copy from backend webtransport.rs
// TODO: rename
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

/// how events should be received
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
        ///
        /// client can only set this to `Pending` and `Disabled`
        status: SyncWebhookStatus,

        /// what events to receive on this transport
        subscribe: ShardKind,
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

/// query parameters when establishing a websocket (or webtransport) sync connection
#[record]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct WebsocketSyncParams {
    pub version: SyncVersion,

    pub compression: Option<SyncCompression>,

    #[serde(default)]
    pub encoding: SyncEncoding,
}
