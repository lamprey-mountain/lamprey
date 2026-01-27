//! api errors

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    application::{Scope, Scopes},
    ChannelId, Permission, RoomId,
};

// TODO: cfg_attr serde
/// an error that may be returned from the api
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ApiError {
    /// human readable error message
    pub message: String,

    /// error code
    pub code: ErrorCode,

    /// errors in the request body
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ErrorField>,

    /// required room permissions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_permissions: Vec<Permission>,

    /// required server permissions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_permissions_server: Vec<Permission>,

    /// required oauth scopes
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_scopes: Vec<Scope>,

    /// unacknowledged warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<Warning>,

    /// moderator-set message for automod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automod_message: Option<String>,

    /// ratelimit that you ran into
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratelimit: Option<Ratelimit>,
}

/// warnings that require forcing
///
/// generally, this means you must pass ?force=true in the url. if you like to
/// live life on the edge, you can always pass ?force.
// maybe require header instead? `X-Force: Warning1, Warning2`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Warning {
    /// this role is applied to one or more room member
    RoleNotEmpty,

    /// this tag is applied to one or more post
    TagNotEmpty,
    // this will revoke view access to existing thread members
    // this will revoke view access to existing rsvpers
    // this will remove all permission overwrites and sync access with parent channel
}

/// an error that may be returned from the sync websocket
#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncError {
    /// invalid sequence number (connection may be too old)
    #[error("invalid sequence number (connection may be too old)")]
    InvalidSeq,

    /// you were sent a `Ping` but didn't respond with a `Pong` in time
    #[error("you were sent a `Ping` but didn't respond with a `Pong` in time")]
    Timeout,

    /// you tried to do something that you can't do
    #[error("you tried to do something that you can't do")]
    Unauthorized,

    /// you tried to do something before sending a `Hello` or `Resume`
    #[error("you tried to do something before sending a `Hello` or `Resume`")]
    Unauthenticated,

    /// you tried to send a `Hello` or `Resume` but were already authenticated
    #[error("you tried to send a `Hello` or `Resume` but were already authenticated")]
    AlreadyAuthenticated,

    /// the token sent in `Hello` or `Resume` is invalid
    #[error("the token sent in `Hello` or `Resume` is invalid")]
    AuthFailure,

    /// you sent data that i couldn't decode. make sure you're encoding payloads as utf-8 json as text.
    #[error(
        "you sent data that i couldn't decode. make sure you're encoding payloads as utf-8 json as text."
    )]
    InvalidData,
    // /// an api error
    // // NOTE: may be removed later
    // #[error("{0}")]
    // Api(#[from] ApiError),
}

/// a field that has an error
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ErrorField {
    /// path to this field inside the request object
    pub key: Vec<String>,

    /// human readable error message
    // TODO: remove this, generate from `ty`.
    // re-add { message: String } to ErrorFieldType::Other
    pub message: String,

    #[serde(flatten)]
    pub ty: ErrorFieldType,
}

/// the type of error in the field
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename = "type")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ErrorFieldType {
    /// this field was required but not specified
    Required,

    /// the specified number is out of range
    Range { min: Option<u64>, max: Option<u64> },

    /// the specified string or array length is out of range
    Length { min: Option<u64>, max: Option<u64> },

    /// the incorrect type was passed
    Type { got: String, expected: String },

    /// some other validation error
    Other,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Ratelimit {
    /// how many seconds to wait until retrying
    pub retry_after: f64,

    /// if this is a global ratelimit
    ///
    /// if false, this only affects this bucket
    pub global: bool,
}

#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ErrorCode {
    /// invalid data was provided
    ///
    /// aka malformed request body, http 400, bad request
    #[error("invalid data was provided")]
    InvalidData,

    /// user is suspended
    #[error("user is suspended")]
    UserSuspended,

    /// missing scopes
    #[error("missing scopes {scopes:?}")]
    MissingScopes { scopes: Scopes },

    /// sudo mode required for this endpoint
    #[error("sudo mode required for this endpoint")]
    SudoRequired,

    // not bot owner
    // user is not a bot
    // bot is not a bridge
    // you can only puppet users of type Puppet
    // you can only puppet your own puppets
    // user is not a puppet

    // missing permissions (Forbidden)
    // slowmode in effect
    // invalid data (populate fields)

    // channel is archived
    // channel is removed
    // you are not the gdm owner
    // only gdms can be upgraded
    // dm/gdm channel missing recipients
    // dms can only be with a single person
    // gdm has too many members
    // can only create dms/gdms outside of rooms
    // channel doesnt have text
    // channel doesnt have voice

    // bitrate is too high
    // cannot set bitrate for non voice thread
    // cannot set user_limit for non voice thread
    // only gdms can have icons
    // icon is not an image

    // /// unknown builtin automod list
    // UnknownAutomodList,

    // /// unknown builtin media scanner
    // UnknownMediaScanner,

    // latest message version cannot be deleted
    // cannot delete that message type
    // cannot edit that message type
    // cannot edit other user's messages
    // maximum number of pinned messages reached
    // invalid message content (must contain content, attachments, or embeds)

    // duplicate media id
    // media already used
    /// unknown room
    // FIXME: the method `as_display` exists for reference `&std::option::Option<Id<MarkerRoom>>`, but its trait bounds were not satisfied
    #[error("unknown room (tried to fetch room with id {bad_room_id:?})")]
    UnknownRoom { bad_room_id: Option<RoomId> },

    /// unknown channel
    #[error("unknown channel (tried to fetch channel with id {bad_channel_id:?})")]
    UnknownChannel { bad_channel_id: Option<ChannelId> },
    // impl unknown thread, message, message version, user, media, invite, application, automod rule, webhook, room member, thread member, ban, email, document branch, document revision

    // calls can only be created in Broadcast channels
    // calls can only be deleted in Broadcast channels

    // your account must have mfa enabled to use this operation
    /// you have angered automod
    ///
    /// - you sent a bad message
    /// - you edited a message to say something bad
    /// - you created a thread with bad words
    /// - your username or profile has something bad
    #[error("you have angered automod")]
    Automod,
    // invalid or expired session (same as AuthFailure?)

    // warning

    // you didn't create this media

    // ratelimited
}

impl ApiError {
    pub fn with_message(code: ErrorCode, message: String) -> Self {
        Self {
            message,
            code,
            fields: vec![],
            required_permissions: vec![],
            required_permissions_server: vec![],
            required_scopes: vec![],
            warnings: vec![],
            automod_message: None,
            ratelimit: None,
        }
    }

    pub fn from_code(code: ErrorCode) -> Self {
        Self {
            message: code.to_string(),
            code,
            fields: vec![],
            required_permissions: vec![],
            required_permissions_server: vec![],
            required_scopes: vec![],
            warnings: vec![],
            automod_message: None,
            ratelimit: None,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

// TODO: use StatusCode enum from http crate
impl ErrorCode {
    pub fn status(&self) -> u16 {
        match self {
            ErrorCode::InvalidData => 400,
            ErrorCode::UserSuspended => 403,
            ErrorCode::MissingScopes { .. } => 403,
            ErrorCode::SudoRequired => 401,
            ErrorCode::UnknownRoom { .. } => 404,
            ErrorCode::UnknownChannel { .. } => 404,
            ErrorCode::Automod => 403,
        }
    }
}
