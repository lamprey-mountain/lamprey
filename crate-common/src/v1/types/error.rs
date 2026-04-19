//! api errors

use http::StatusCode;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{application::Scope, Permission};

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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub fields: Vec<ErrorField>,

    /// required room permissions
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_permissions: Vec<Permission>,

    /// required server permissions
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_permissions_server: Vec<Permission>,

    /// required oauth scopes
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_scopes: Vec<Scope>,

    /// unacknowledged warnings
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub warnings: Vec<Warning>,

    /// moderator-set message for automod
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub automod_message: Option<String>,

    /// ratelimit that you ran into
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
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
}

/// a field that has an error
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ErrorField {
    /// path to this field inside the request object
    pub key: Vec<String>,

    /// human readable error message
    // TODO: remove this, generate from `ty`?
    // re-add { message: String } to ErrorFieldType::Other
    pub message: String,

    #[cfg_attr(feature = "serde", serde(flatten))]
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
    // NOTE: should these be usize?
    Range { min: Option<u64>, max: Option<u64> },

    /// the specified string or array length is out of range
    // NOTE: should these be usize?
    Length { min: Option<u64>, max: Option<u64> },

    /// the incorrect type was passed
    Type { got: String, expected: String },

    /// some other validation error
    Other,
}

impl ErrorFieldType {
    /// construct a `ErrorFieldType::Length`
    pub fn length(min: u64, max: u64) -> Self {
        Self::Length {
            min: Some(min),
            max: Some(max),
        }
    }
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
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

    /// missing oauth scope(s)
    #[error("missing oauth scope(s)")]
    MissingScopes,

    /// invalid oauth scope(s)
    #[error("invalid oauth scope(s)")]
    InvalidScope,

    /// sudo mode required for this endpoint
    #[error("sudo mode required for this endpoint")]
    SudoRequired,

    /// mfa required for this action
    #[error("mfa required for this action")]
    MfaRequired,

    /// you are missing permissions
    #[error("missing permissions")]
    MissingPermissions,

    /// thread is archived
    #[error("thread is archived")]
    ThreadArchived,

    /// thread is removed
    #[error("thread is removed")]
    ThreadRemoved,

    /// thread is locked
    #[error("thread is locked")]
    ThreadLocked,

    /// channel is archived
    #[error("channel is archived")]
    ChannelArchived,

    /// channel is removed
    #[error("channel is removed")]
    ChannelRemoved,

    /// cannot delete latest message version
    #[error("cannot delete latest message version")]
    CannotDeleteLatestMessageVersion,

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
    /// media already used
    #[error("media already used")]
    MediaAlreadyUsed,

    /// duplicate media id
    #[error("duplicate media id")]
    DuplicateMediaId,

    /// unknown room
    #[error("unknown room")]
    UnknownRoom,

    /// unknown channel
    #[error("unknown channel")]
    UnknownChannel,

    /// unknown thread
    #[error("unknown thread")]
    UnknownThread,

    /// unknown message
    #[error("unknown message")]
    UnknownMessage,

    /// unknown message version
    #[error("unknown message version")]
    UnknownMessageVersion,

    /// unknown user
    #[error("unknown user")]
    UnknownUser,

    /// unknown media
    #[error("unknown media")]
    UnknownMedia,

    /// unknown invite
    #[error("unknown invite")]
    UnknownInvite,

    /// unknown application
    #[error("unknown application")]
    UnknownApplication,

    /// unknown automod rule
    #[error("unknown automod rule")]
    UnknownAutomodRule,

    /// unknown webhook
    #[error("unknown webhook")]
    UnknownWebhook,

    /// unknown room template
    #[error("unknown room template")]
    UnknownRoomTemplate,

    /// unknown room member
    #[error("unknown room member")]
    UnknownRoomMember,

    /// unknown thread member
    #[error("unknown thread member")]
    UnknownThreadMember,

    /// unknown room ban
    #[error("unknown room ban")]
    UnknownRoomBan,

    /// unknown user email
    #[error("unknown user email")]
    UnknownUserEmail,

    /// unknown document branch
    #[error("unknown document branch")]
    UnknownDocumentBranch,

    /// unknown document revision
    #[error("unknown document revision")]
    UnknownDocumentRevision,

    /// unknown emoji
    #[error("unknown emoji")]
    UnknownEmoji,

    /// unknown session
    #[error("unknown session")]
    UnknownSession,

    /// unknown role
    #[error("unknown role")]
    UnknownRole,

    /// unknown calendar event
    #[error("unknown calendar event")]
    UnknownCalendarEvent,

    /// unknown document
    #[error("unknown document")]
    UnknownDocument,

    /// unknown wiki
    #[error("unknown wiki")]
    UnknownWiki,

    /// unknown tag
    #[error("unknown tag")]
    UnknownTag,

    /// unknown notification
    #[error("unknown notification")]
    UnknownNotification,

    /// unknown reaction
    #[error("unknown reaction")]
    UnknownReaction,

    /// unknown connection
    #[error("unknown connection")]
    UnknownConnection,

    /// unknown oauth2 client
    #[error("unknown oauth2 client")]
    UnknownOauth2Client,

    /// unknown voice channel
    #[error("unknown voice channel")]
    UnknownVoiceChannel,

    /// unknown dm
    #[error("unknown dm")]
    UnknownDm,

    /// cannot set strip_exif to false once it has been set to true
    #[error("cannot set strip_exif to false once it has been set to true")]
    CannotUnsetStripExif,

    // TODO: rename, merge fix up error codes below
    /// cannot act on behalf of other users
    #[error("cannot act on behalf of other users")]
    CannotActOnBehalfOfOthers,

    // this should probably be in `fields`
    /// permission conflict (a permission cannot be both allowed and denied)
    #[error("permission conflict (a permission cannot be both allowed and denied)")]
    PermissionConflict,

    /// insufficient rank
    #[error("insufficient rank")]
    InsufficientRank,

    /// insufficient rank to manage this user
    #[error("insufficient rank to manage this user")]
    InsufficientRankToManageUser,

    /// cannot modify the default (@everyone) role
    #[error("cannot modify the default (@everyone) role")]
    CannotModifyDefaultRole,

    // this should probably be in `fields`?
    /// dm thread is missing recipients
    #[error("dm thread is missing recipients")]
    DmThreadMissingRecipients,

    // this should probably be in `fields`?
    /// dm threads can only be with a single person
    #[error("dm threads can only be with a single person")]
    DmThreadSinglePersonOnly,

    /// gdm thread is missing recipients
    #[error("gdm thread is missing recipients")]
    GdmThreadMissingRecipients,

    /// group dm has too many members
    #[error("group dm has too many members")]
    GdmTooManyMembers,

    // rename to generic "can't add user" code?
    /// you must be friends with all recipients to create a group dm
    #[error("you must be friends with all recipients to create a group dm")]
    GdmRequiresFriend,

    /// can only create a dm/gdm thread outside of a room
    #[error("can only create a dm/gdm thread outside of a room")]
    DmGdmOnlyOutsideRoom,

    // this should probably be in `fields`
    /// bitrate is too high
    #[error("bitrate is too high")]
    BitrateTooHigh,

    // rename to "not a voice channel"
    /// cannot set bitrate for non voice thread
    #[error("cannot set bitrate for non voice thread")]
    CannotSetBitrateForNonVoiceThread,

    /// cannot set user_limit for non voice thread
    #[error("cannot set user_limit for non voice thread")]
    CannotSetUserLimitForNonVoiceThread,

    /// only gdm threads can have icons
    #[error("only gdm threads can have icons")]
    OnlyGdmCanHaveIcons,

    /// media not an image
    #[error("media not an image")]
    MediaNotAnImage,

    /// invalid parent channel type
    #[error("invalid parent channel type")]
    InvalidParentChannelType,

    /// owner_id cannot be changed via this endpoint
    #[error("owner_id cannot be changed via this endpoint")]
    OwnerIdCannotBeChanged,

    /// channel doesnt have text
    #[error("channel doesnt have text")]
    ChannelDoesntHaveText,

    /// channel doesnt have voice
    #[error("channel doesnt have voice")]
    ChannelDoesntHaveVoice,

    /// cannot edit thread member list
    #[error("cannot edit thread member list")]
    CannotEditThreadMemberList,

    /// invalid thread type
    #[error("invalid thread type")]
    InvalidThreadType,

    /// cant delete that message
    #[error("cant delete that message")]
    CantDeleteThatMessage,

    // merge with CantDeleteThatMessage?
    /// cant delete that message type
    #[error("cant delete that message type")]
    CantDeleteThatMessageType,

    /// maximum number of pinned messages reached
    #[error("maximum number of pinned messages reached")]
    MaxPinsReached,

    /// only group dms can be upgraded
    #[error("only group dms can be upgraded")]
    OnlyGdmCanUpgrade,

    /// you are not the thread owner
    #[error("you are not the thread owner")]
    NotThreadOwner,

    /// thread is already in a room
    #[error("thread is already in a room")]
    ThreadAlreadyInRoom,

    /// guests cannot join public rooms
    #[error("guests cannot join public rooms")]
    GuestsCannotJoinPublicRooms,

    /// you are banned
    #[error("you are banned")]
    YouAreBanned,

    /// can't add that user
    #[error("can't add that user")]
    CantAddThatUser,

    /// only bots can use this
    #[error("only bots can use this")]
    OnlyBotsCanUseThis,

    /// bot is not a bridge
    #[error("bot is not a bridge")]
    BotIsNotABridge,

    /// not puppet owner
    #[error("not puppet owner")]
    NotPuppetOwner,

    // merge with InsufficientRank?
    /// cannot add role above your role
    #[error("cannot add role above your role")]
    CannotAddRoleAboveYourRole,

    // merge with InsufficientRank?
    /// cannot remove role above your role
    #[error("cannot remove role above your role")]
    CannotRemoveRoleAboveYourRole,

    /// you aren't the room owner
    #[error("you aren't the room owner")]
    NotRoomOwner,

    /// room owner must have mfa enabled
    #[error("room owner must have mfa enabled")]
    RoomOwnerMustHaveMfa,

    // rename -> cannot manage members?
    /// cannot kick people from the server room
    #[error("cannot kick people from the server room")]
    CannotKickFromServerRoom,

    /// room owner cannot leave the room
    #[error("room owner cannot leave the room")]
    RoomOwnerCannotLeave,

    // merge with InsufficientRank?
    /// cannot ban room owner
    #[error("cannot ban room owner")]
    CannotBanRoomOwner,

    // remove? this is MissingPermissions
    /// cannot add roles to yourself
    #[error("cannot add roles to yourself")]
    CannotAddRolesToYourself,

    /// user is not a guest account
    #[error("user is not a guest account")]
    UserIsNotAGuestAccount,

    /// add an auth method first
    #[error("add an auth method first")]
    AddAuthMethodFirst,

    /// cannot create invite for server room
    // remove?
    #[error("cannot create invite for server room")]
    CannotCreateInviteForServerRoom,

    /// cannot add roles to this invite type
    #[error("cannot add roles to this invite type")]
    CannotAddRolesToInvite,

    /// channel is not in a room or gdm
    #[error("channel is not in a room or gdm")]
    ChannelNotInRoomOrGdm,

    /// channel is not in a room
    #[error("channel is not in a room")]
    ChannelNotInRoom,

    /// guests cannot create server invites
    #[error("guests cannot create server invites")]
    GuestsCannotCreateServerInvites,

    /// guests cannot list server invites
    #[error("guests cannot list server invites")]
    GuestsCannotListServerInvites,

    // specify that these errors are for oauth
    /// unknown response type
    #[error("unknown response type")]
    UnknownResponseType,

    /// bad redirect uri
    #[error("bad redirect uri")]
    BadRedirectUri,

    /// no redirect uri configured
    #[error("no redirect uri configured")]
    NoRedirectUriConfigured,

    /// invalid client id
    #[error("invalid client id")]
    InvalidClientId,

    /// missing code
    #[error("missing code")]
    MissingCode,

    /// missing redirect uri
    #[error("missing redirect uri")]
    MissingRedirectUri,

    /// missing code verifier
    #[error("missing code verifier")]
    MissingCodeVerifier,

    /// unsupported code challenge method
    #[error("unsupported code challenge method")]
    UnsupportedCodeChallengeMethod,

    /// missing refresh token
    #[error("missing refresh token")]
    MissingRefreshToken,

    /// unsupported grant type
    #[error("unsupported grant type")]
    UnsupportedGrantType,

    /// not an oauth token
    #[error("not an oauth token")]
    NotAnOauthToken,

    /// can only create user on your own server
    #[error("can only create user on your own server")]
    CanOnlyCreateUserOnOwnServer,

    /// can only sync for this server
    #[error("can only sync for this server")]
    CanOnlySyncForThisServer,

    /// platform name is required for bridge
    #[error("platform name is required for bridge")]
    PlatformNameRequiredForBridge,

    /// cant create that user
    #[error("cant create that user")]
    CantCreateThatUser,

    // invalid field?
    /// field is missing name
    #[error("field is missing name")]
    FieldMissingName,

    // remove?
    /// unknown field
    #[error("unknown field")]
    UnknownField,

    /// no data
    #[error("no data")]
    NoData,

    /// channel is not a calendar
    #[error("channel is not a calendar")]
    ChannelIsNotACalendar,

    /// cannot rsvp other people
    #[error("cannot rsvp other people")]
    CannotRsvpOtherPeople,

    /// webhook not in a room
    #[error("webhook not in a room")]
    WebhookNotInRoom,

    /// cannot set permissions on this channel type
    #[error("cannot set permissions on this channel type")]
    CannotSetPermissionsOnThisChannelType,

    /// cannot set permissions on parent channel of this type
    #[error("cannot set permissions on parent channel of this type")]
    CannotSetPermissionsOnParentChannelOfType,

    /// cannot remove last auth method
    #[error("cannot remove last auth method")]
    CannotRemoveLastAuthMethod,

    /// cannot dm this user
    #[error("cannot dm this user")]
    CannotDmThisUser,

    /// dms not allowed from this user
    #[error("dms not allowed from this user")]
    DmsNotAllowedFromThisUser,

    /// bots cannot use this endpoint
    #[error("bots cannot use this endpoint")]
    BotsCannotUseThisEndpoint,

    /// channel does not support tags
    #[error("channel does not support tags")]
    ChannelDoesNotSupportTags,

    /// failed to encode metrics
    #[error("failed to encode metrics")]
    FailedToEncodeMetrics,

    /// cannot move to different room
    #[error("cannot move to different room")]
    CannotMoveToDifferentRoom,

    /// cannot move to thread without voice
    #[error("cannot move to thread without voice")]
    CannotMoveToThreadWithoutVoice,

    /// not connected to any thread
    #[error("not connected to any thread")]
    NotConnectedToAnyThread,

    /// cannot move to thread in different room
    #[error("cannot move to thread in different room")]
    CannotMoveToThreadInDifferentRoom,

    /// cannot close default branch
    #[error("cannot close default branch")]
    CannotCloseDefaultBranch,

    /// cannot merge default branch
    #[error("cannot merge default branch")]
    CannotMergeDefaultBranch,

    /// branch has no parent
    #[error("branch has no parent")]
    BranchHasNoParent,

    /// cannot tag another tag
    #[error("cannot tag another tag")]
    CannotTagAnotherTag,

    /// cannot friend this user
    #[error("cannot friend this user")]
    CannotFriendThisUser,

    /// friend requests are paused
    #[error("friend requests are paused")]
    FriendRequestsPaused,

    /// unblock this user first
    #[error("unblock this user first")]
    UnblockUserFirst,

    /// sudo session expired
    #[error("sudo session expired")]
    SudoSessionExpired,

    /// invalid or expired code
    #[error("invalid or expired code")]
    InvalidOrExpiredCode,

    /// totp not initialized
    #[error("totp not initialized")]
    TotpNotInitialized,

    /// totp already enabled
    #[error("totp already enabled")]
    TotpAlreadyEnabled,

    /// invalid totp code
    #[error("invalid totp code")]
    InvalidTotpCode,

    /// totp not enabled
    #[error("totp not enabled")]
    TotpNotEnabled,

    /// already authenticated
    #[error("already authenticated")]
    AlreadyAuthenticated,

    /// invalid password
    #[error("invalid password")]
    InvalidPassword,

    /// not bot owner
    #[error("not bot owner")]
    NotBotOwner,

    /// user is not a bot
    #[error("user is not a bot")]
    UserIsNotABot,

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
    /// ratelimited
    #[error("ratelimited")]
    Ratelimit,

    /// not found
    #[error("not found")]
    NotFound,

    /// unimplemented
    #[error("unimplemented")]
    Unimplemented,

    /// internal error
    #[error("internal")]
    Internal,

    /// only the message author can manage flume
    #[error("only the message author can manage flume")]
    OnlyMessageAuthorCanManageFlume,

    /// flume is committed and cannot be modified
    #[error("flume is committed and cannot be modified")]
    FlumeCommitted,

    /// message exists but has no associated flume
    #[error("message does not have a flume")]
    MessageDoesntHaveFlume,
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

impl ErrorCode {
    /// get the http status code for this error
    pub fn status(&self) -> StatusCode {
        match self {
            ErrorCode::InvalidData => StatusCode::BAD_REQUEST,
            ErrorCode::UserSuspended => StatusCode::FORBIDDEN,
            ErrorCode::MissingScopes => StatusCode::FORBIDDEN,
            ErrorCode::SudoRequired => StatusCode::UNAUTHORIZED,
            ErrorCode::UnknownRoom => StatusCode::NOT_FOUND,
            ErrorCode::UnknownChannel => StatusCode::NOT_FOUND,
            ErrorCode::UnknownThread => StatusCode::NOT_FOUND,
            ErrorCode::UnknownMessage => StatusCode::NOT_FOUND,
            ErrorCode::UnknownMessageVersion => StatusCode::NOT_FOUND,
            ErrorCode::UnknownUser => StatusCode::NOT_FOUND,
            ErrorCode::UnknownMedia => StatusCode::NOT_FOUND,
            ErrorCode::UnknownInvite => StatusCode::NOT_FOUND,
            ErrorCode::UnknownApplication => StatusCode::NOT_FOUND,
            ErrorCode::UnknownAutomodRule => StatusCode::NOT_FOUND,
            ErrorCode::UnknownWebhook => StatusCode::NOT_FOUND,
            ErrorCode::UnknownRoomTemplate => StatusCode::NOT_FOUND,
            ErrorCode::UnknownRoomMember => StatusCode::NOT_FOUND,
            ErrorCode::UnknownThreadMember => StatusCode::NOT_FOUND,
            ErrorCode::UnknownRoomBan => StatusCode::NOT_FOUND,
            ErrorCode::UnknownUserEmail => StatusCode::NOT_FOUND,
            ErrorCode::UnknownDocumentBranch => StatusCode::NOT_FOUND,
            ErrorCode::UnknownDocumentRevision => StatusCode::NOT_FOUND,
            ErrorCode::UnknownEmoji => StatusCode::NOT_FOUND,
            ErrorCode::UnknownSession => StatusCode::NOT_FOUND,
            ErrorCode::UnknownRole => StatusCode::NOT_FOUND,
            ErrorCode::UnknownCalendarEvent => StatusCode::NOT_FOUND,
            ErrorCode::UnknownDocument => StatusCode::NOT_FOUND,
            ErrorCode::UnknownWiki => StatusCode::NOT_FOUND,
            ErrorCode::UnknownTag => StatusCode::NOT_FOUND,
            ErrorCode::UnknownNotification => StatusCode::NOT_FOUND,
            ErrorCode::UnknownReaction => StatusCode::NOT_FOUND,
            ErrorCode::UnknownConnection => StatusCode::NOT_FOUND,
            ErrorCode::UnknownOauth2Client => StatusCode::NOT_FOUND,
            ErrorCode::UnknownVoiceChannel => StatusCode::NOT_FOUND,
            ErrorCode::UnknownDm => StatusCode::NOT_FOUND,
            ErrorCode::Automod => StatusCode::FORBIDDEN,
            ErrorCode::MissingPermissions => StatusCode::FORBIDDEN,
            ErrorCode::CannotUnsetStripExif => StatusCode::BAD_REQUEST,
            ErrorCode::CannotActOnBehalfOfOthers => StatusCode::FORBIDDEN,
            ErrorCode::ThreadArchived => StatusCode::BAD_REQUEST,
            ErrorCode::ThreadRemoved => StatusCode::NOT_FOUND,
            ErrorCode::ThreadLocked => StatusCode::FORBIDDEN,
            ErrorCode::ChannelArchived => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelRemoved => StatusCode::NOT_FOUND,
            ErrorCode::MfaRequired => StatusCode::FORBIDDEN,
            ErrorCode::PermissionConflict => StatusCode::BAD_REQUEST,
            ErrorCode::InsufficientRank => StatusCode::FORBIDDEN,
            ErrorCode::InsufficientRankToManageUser => StatusCode::FORBIDDEN,
            ErrorCode::CannotModifyDefaultRole => StatusCode::BAD_REQUEST,
            ErrorCode::DmThreadMissingRecipients => StatusCode::BAD_REQUEST,
            ErrorCode::DmThreadSinglePersonOnly => StatusCode::BAD_REQUEST,
            ErrorCode::GdmThreadMissingRecipients => StatusCode::BAD_REQUEST,
            ErrorCode::GdmTooManyMembers => StatusCode::BAD_REQUEST,
            ErrorCode::GdmRequiresFriend => StatusCode::FORBIDDEN,
            ErrorCode::DmGdmOnlyOutsideRoom => StatusCode::BAD_REQUEST,
            ErrorCode::BitrateTooHigh => StatusCode::BAD_REQUEST,
            ErrorCode::CannotSetBitrateForNonVoiceThread => StatusCode::BAD_REQUEST,
            ErrorCode::CannotSetUserLimitForNonVoiceThread => StatusCode::BAD_REQUEST,
            ErrorCode::OnlyGdmCanHaveIcons => StatusCode::BAD_REQUEST,
            ErrorCode::MediaNotAnImage => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::InvalidParentChannelType => StatusCode::BAD_REQUEST,
            ErrorCode::OwnerIdCannotBeChanged => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelDoesntHaveText => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelDoesntHaveVoice => StatusCode::BAD_REQUEST,
            ErrorCode::CannotEditThreadMemberList => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidThreadType => StatusCode::BAD_REQUEST,
            ErrorCode::CantDeleteThatMessage => StatusCode::FORBIDDEN,
            ErrorCode::CantDeleteThatMessageType => StatusCode::FORBIDDEN,
            ErrorCode::CannotDeleteLatestMessageVersion => StatusCode::BAD_REQUEST,
            ErrorCode::MaxPinsReached => StatusCode::BAD_REQUEST,
            ErrorCode::OnlyGdmCanUpgrade => StatusCode::BAD_REQUEST,
            ErrorCode::NotThreadOwner => StatusCode::FORBIDDEN,
            ErrorCode::ThreadAlreadyInRoom => StatusCode::BAD_REQUEST,
            ErrorCode::GuestsCannotJoinPublicRooms => StatusCode::FORBIDDEN,
            ErrorCode::YouAreBanned => StatusCode::FORBIDDEN,
            ErrorCode::CantAddThatUser => StatusCode::BAD_REQUEST,
            ErrorCode::OnlyBotsCanUseThis => StatusCode::FORBIDDEN,
            ErrorCode::BotIsNotABridge => StatusCode::FORBIDDEN,
            ErrorCode::NotPuppetOwner => StatusCode::FORBIDDEN,
            ErrorCode::CannotAddRoleAboveYourRole => StatusCode::FORBIDDEN,
            ErrorCode::CannotRemoveRoleAboveYourRole => StatusCode::FORBIDDEN,
            ErrorCode::NotRoomOwner => StatusCode::FORBIDDEN,
            ErrorCode::RoomOwnerMustHaveMfa => StatusCode::FORBIDDEN,
            ErrorCode::CannotKickFromServerRoom => StatusCode::BAD_REQUEST,
            ErrorCode::RoomOwnerCannotLeave => StatusCode::BAD_REQUEST,
            ErrorCode::CannotBanRoomOwner => StatusCode::FORBIDDEN,
            ErrorCode::CannotAddRolesToYourself => StatusCode::FORBIDDEN,
            ErrorCode::UserIsNotAGuestAccount => StatusCode::BAD_REQUEST,
            ErrorCode::AddAuthMethodFirst => StatusCode::BAD_REQUEST,
            ErrorCode::CannotCreateInviteForServerRoom => StatusCode::BAD_REQUEST,
            ErrorCode::CannotAddRolesToInvite => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelNotInRoomOrGdm => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelNotInRoom => StatusCode::BAD_REQUEST,
            ErrorCode::GuestsCannotCreateServerInvites => StatusCode::FORBIDDEN,
            ErrorCode::GuestsCannotListServerInvites => StatusCode::FORBIDDEN,
            ErrorCode::UnknownResponseType => StatusCode::BAD_REQUEST,
            ErrorCode::BadRedirectUri => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidScope => StatusCode::BAD_REQUEST,
            ErrorCode::NoRedirectUriConfigured => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidClientId => StatusCode::BAD_REQUEST,
            ErrorCode::MissingCode => StatusCode::BAD_REQUEST,
            ErrorCode::MissingRedirectUri => StatusCode::BAD_REQUEST,
            ErrorCode::MissingCodeVerifier => StatusCode::BAD_REQUEST,
            ErrorCode::UnsupportedCodeChallengeMethod => StatusCode::BAD_REQUEST,
            ErrorCode::MissingRefreshToken => StatusCode::BAD_REQUEST,
            ErrorCode::UnsupportedGrantType => StatusCode::BAD_REQUEST,
            ErrorCode::NotAnOauthToken => StatusCode::BAD_REQUEST,
            ErrorCode::CanOnlyCreateUserOnOwnServer => StatusCode::FORBIDDEN,
            ErrorCode::CanOnlySyncForThisServer => StatusCode::FORBIDDEN,
            ErrorCode::PlatformNameRequiredForBridge => StatusCode::BAD_REQUEST,
            ErrorCode::CantCreateThatUser => StatusCode::BAD_REQUEST,
            ErrorCode::FieldMissingName => StatusCode::BAD_REQUEST,
            ErrorCode::UnknownField => StatusCode::BAD_REQUEST,
            ErrorCode::NoData => StatusCode::BAD_REQUEST,
            ErrorCode::ChannelIsNotACalendar => StatusCode::BAD_REQUEST,
            ErrorCode::CannotRsvpOtherPeople => StatusCode::FORBIDDEN,
            ErrorCode::WebhookNotInRoom => StatusCode::BAD_REQUEST,
            ErrorCode::CannotSetPermissionsOnThisChannelType => StatusCode::BAD_REQUEST,
            ErrorCode::CannotSetPermissionsOnParentChannelOfType => StatusCode::BAD_REQUEST,
            ErrorCode::CannotRemoveLastAuthMethod => StatusCode::BAD_REQUEST,
            ErrorCode::CannotDmThisUser => StatusCode::FORBIDDEN,
            ErrorCode::DmsNotAllowedFromThisUser => StatusCode::FORBIDDEN,
            ErrorCode::BotsCannotUseThisEndpoint => StatusCode::FORBIDDEN,
            ErrorCode::ChannelDoesNotSupportTags => StatusCode::BAD_REQUEST,
            ErrorCode::FailedToEncodeMetrics => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::CannotMoveToDifferentRoom => StatusCode::BAD_REQUEST,
            ErrorCode::CannotMoveToThreadWithoutVoice => StatusCode::BAD_REQUEST,
            ErrorCode::NotConnectedToAnyThread => StatusCode::BAD_REQUEST,
            ErrorCode::CannotMoveToThreadInDifferentRoom => StatusCode::BAD_REQUEST,
            ErrorCode::CannotCloseDefaultBranch => StatusCode::BAD_REQUEST,
            ErrorCode::CannotMergeDefaultBranch => StatusCode::BAD_REQUEST,
            ErrorCode::BranchHasNoParent => StatusCode::BAD_REQUEST,
            ErrorCode::CannotTagAnotherTag => StatusCode::BAD_REQUEST,
            ErrorCode::CannotFriendThisUser => StatusCode::FORBIDDEN,
            ErrorCode::FriendRequestsPaused => StatusCode::BAD_REQUEST,
            ErrorCode::UnblockUserFirst => StatusCode::FORBIDDEN,
            ErrorCode::SudoSessionExpired => StatusCode::UNAUTHORIZED,
            ErrorCode::InvalidOrExpiredCode => StatusCode::BAD_REQUEST,
            ErrorCode::TotpNotInitialized => StatusCode::BAD_REQUEST,
            ErrorCode::TotpAlreadyEnabled => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidTotpCode => StatusCode::BAD_REQUEST,
            ErrorCode::TotpNotEnabled => StatusCode::BAD_REQUEST,
            ErrorCode::AlreadyAuthenticated => StatusCode::FORBIDDEN,
            ErrorCode::InvalidPassword => StatusCode::UNAUTHORIZED,
            ErrorCode::NotBotOwner => StatusCode::FORBIDDEN,
            ErrorCode::UserIsNotABot => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Ratelimit => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::DuplicateMediaId => StatusCode::BAD_REQUEST,
            ErrorCode::MediaAlreadyUsed => StatusCode::CONFLICT,
            ErrorCode::OnlyMessageAuthorCanManageFlume => StatusCode::FORBIDDEN,
            ErrorCode::FlumeCommitted => StatusCode::FORBIDDEN,
            ErrorCode::MessageDoesntHaveFlume => StatusCode::NOT_FOUND,
        }
    }
}

impl SyncError {
    /// get the websocket close code for this error
    pub fn code(&self) -> u16 {
        match self {
            SyncError::InvalidData => 1007,
            SyncError::Unauthorized => 3003,
            SyncError::Unauthenticated => 3000,
            SyncError::Timeout => 3008,
            SyncError::AuthFailure => 4004,
            SyncError::AlreadyAuthenticated => 4005,
            SyncError::InvalidSeq => 4007,
        }
    }
}
