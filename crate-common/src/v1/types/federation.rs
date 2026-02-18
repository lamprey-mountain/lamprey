use uuid::Uuid;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{misc::Time, MessageSync, Session, User, UserId};

/// A hostname, used to identify a server
// NOTE: do i really want to use this as an id?
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples("example.com")))]
pub struct Hostname(pub String);

/// a piece of content on a remote server
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Remote {
    /// the id of this resource on the origin server
    pub origin_id: Uuid,

    /// the hostname of the server
    pub hostname: String,
}

/// a server's signing key
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerKey {
    /// the key algorithm
    ///
    /// always the string `ed25519`
    pub alg: String,

    /// public key
    ///
    /// base64 url safe unpadded
    pub pubkey: String,

    /// random data to sign
    ///
    /// base64 url safe unpadded
    pub nonce: String,

    /// the signature
    ///
    /// the bytes that were signed: nonce || "\xff" || pubkey || "\xff" | hostname
    ///
    /// base64 url safe unpadded
    // NOTE: "\xff" is used because utf8 cant contain that char, but since nonce/pubkey are b64 and domain is ascii i could probably use some other char
    pub signature: String,

    /// when this key expires
    ///
    /// maximum Date + 72h, should be Date + 48h and rotated every 24h
    // NOTE: should i require more frequent rotation?
    pub expires_at: Time,
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

/// response for creating a user on a federated server
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerUserCreate {
    /// the user that was created
    pub user: User,

    /// an authenticated session for the user
    pub session: Session,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ServerSyncHandleRequest {
    /// the sync events
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 1024))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub events: Vec<MessageSync>,

    /// sequence id for resuming
    pub seq: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ServerUserCreateRequest {
    /// the id of the user on the requesting server
    ///
    /// used to deduplicate users
    pub local_id: UserId,

    /// display name
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    /// about/bio
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// if this is a remote bot
    pub bot: bool,

    /// if this is for the service itself. usually paired with bot: true
    pub system: bool,
}
