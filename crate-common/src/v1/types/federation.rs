use std::ops::Deref;

use url::Url;
use uuid::Uuid;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    error::{ApiError, ErrorCode},
    misc::Time,
    MessageSync,
};

pub mod signing;

/// A hostname, used to identify a server
// NOTE: do i really want to use this as an id?
// TODO: rename to ServerId? or ServerName?
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples("example.com")))]
pub struct Hostname(pub String);

#[cfg(feature = "validator")]
impl validator::Validate for Hostname {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        if crate::util::is_valid_hostname(&self.0) {
            Ok(())
        } else {
            let mut errors = validator::ValidationErrors::new();
            errors.add("0", validator::ValidationError::new("invalid_hostname"));
            Err(errors)
        }
    }
}

impl std::str::FromStr for Hostname {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Hostname::new(s.to_string())
    }
}

/// a piece of content on a remote server
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Remote {
    /// the id of this resource on the origin server
    pub origin_id: Uuid,

    /// the hostname of the server
    pub hostname: Hostname,
    // TODO: add
    // /// the epoch that this remote resource was fetched during
    // ///
    // /// if `item.epoch != server.sync_epoch`, this is stale and should be refetched
    // pub epoch: RemoteEpoch,
}

/// monotonic counter that increments every time sync fails/disconnects
///
/// intended to invalidate cache
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RemoteEpoch(pub u64);

// NOTE: maybe i could use ChannelSeq/ChannelSync stuff for syncing too?
// pub enum RemoteEpoch2 {
//     Channel(ChannelSeq),
//     Room(RoomSeq),
//     Global(u64),
// }

// TODO: more type safety?
// pub struct Remote<M: Marker> {
//     pub origin_id: Id<M>,
//     pub hostname: Hostname,
// }

/// a server's signing key
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerKey {
    /// the key algorithm
    pub alg: ServerKeyAlgorithm,

    /// public key
    ///
    /// base64 url safe unpadded
    // TODO: use Binary
    pub pubkey: String,

    /// random data to sign
    ///
    /// base64 url safe unpadded
    // TODO: use Binary
    pub nonce: String,

    /// the signature
    ///
    /// the bytes that were signed: nonce || pubkey || hostname
    ///
    /// base64 url safe unpadded
    // TODO: use Binary
    pub signature: String,

    /// when this key expires
    ///
    /// maximum Date + 72h, should be Date + 48h and rotated every 24h
    // NOTE: should i require more frequent rotation?
    pub expires_at: Time,
}

/// the algorithm to sign requests with
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ServerKeyAlgorithm {
    Ed25519,
}

/// A collection of server keys for a specific hostname
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerKeys {
    /// The hostname these keys belong to
    pub hostname: String,

    /// The list of keys for this hostname
    pub keys: Vec<ServerKey>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "op"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum FederationMessageSync {
    Sync {
        /// the data for this sync event
        data: Box<MessageSync>,
    },

    /// the target server has lagged behind too far
    ///
    /// maybe its because you processed events too slowly, or your server went
    /// offline. either way, you should bump your epoch and start fetching stuff
    /// from scratch.
    Lagged,

    /// requesting a disconnect. target server should no longer post syncs to
    /// the requesting server.
    Disconnect,
    // // what would the errors be?
    // /// an error occured
    // Error { ... },

    // maybe a requesting server is "going offline" event
}

/// a batch of sync events being pushed to a server
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ServerSyncRequest {
    /// the events for this sync event
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub events: Vec<MessageSync>,
}

/// a response to a server sync request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ServerSyncResponse {
    // /// how much time to delay until sending the next batch, in milliseconds
    // ///
    // /// this is to prevent servers from being overloaded
    // pub timeout: u64,

    // TODO: tell sender its lagged?
    // pub lagged: bool,

    // TODO: tell sender to disconnect?
    // TODO: if lagged or disconnected, should i return some different http status code?
}

/// response to a server connect request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerConnectResponse {
    // TODO: success
    // TODO: epoch?
}

/// response to a server ping request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerPingResponse {
    /// whether this is in response to a server authenticated request
    pub federated: bool,
}

/// lamprey mountain's well know response
///
/// response to `GET /.well-known/lamprey-mountain`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct WellKnown {
    pub api_url: Url,
    pub cdn_url: Url,
}

impl Deref for Hostname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hostname {
    pub fn new(s: String) -> Result<Self, ApiError> {
        if crate::util::is_valid_hostname(&s) {
            Ok(Self(s))
        } else {
            Err(ApiError::with_message(
                ErrorCode::InvalidData,
                format!("invalid hostname: {}", s),
            ))
        }
    }
}

impl std::fmt::Display for Hostname {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
