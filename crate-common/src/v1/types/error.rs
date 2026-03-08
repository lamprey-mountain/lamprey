//! api errors

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

    // duplicate media id
    // media already used
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
    /// get the http status code for this error
    pub fn status(&self) -> u16 {
        match self {
            ErrorCode::InvalidData => 400,
            ErrorCode::UserSuspended => 403,
            ErrorCode::MissingScopes => 403,
            ErrorCode::SudoRequired => 401,
            ErrorCode::UnknownRoom => 404,
            ErrorCode::UnknownChannel => 404,
            ErrorCode::UnknownThread => 404,
            ErrorCode::UnknownMessage => 404,
            ErrorCode::UnknownMessageVersion => 404,
            ErrorCode::UnknownUser => 404,
            ErrorCode::UnknownMedia => 404,
            ErrorCode::UnknownInvite => 404,
            ErrorCode::UnknownApplication => 404,
            ErrorCode::UnknownAutomodRule => 404,
            ErrorCode::UnknownWebhook => 404,
            ErrorCode::UnknownRoomTemplate => 404,
            ErrorCode::UnknownRoomMember => 404,
            ErrorCode::UnknownThreadMember => 404,
            ErrorCode::UnknownRoomBan => 404,
            ErrorCode::UnknownUserEmail => 404,
            ErrorCode::UnknownDocumentBranch => 404,
            ErrorCode::UnknownDocumentRevision => 404,
            ErrorCode::UnknownEmoji => 404,
            ErrorCode::UnknownSession => 404,
            ErrorCode::UnknownRole => 404,
            ErrorCode::UnknownCalendarEvent => 404,
            ErrorCode::UnknownDocument => 404,
            ErrorCode::UnknownWiki => 404,
            ErrorCode::UnknownTag => 404,
            ErrorCode::UnknownNotification => 404,
            ErrorCode::UnknownReaction => 404,
            ErrorCode::UnknownConnection => 404,
            ErrorCode::Automod => 403,
            ErrorCode::MissingPermissions => 403,
            ErrorCode::CannotUnsetStripExif => 400,
            ErrorCode::CannotActOnBehalfOfOthers => 403,
            ErrorCode::ThreadArchived => 400,
            ErrorCode::ThreadRemoved => 404,
            ErrorCode::MfaRequired => 403,
            ErrorCode::PermissionConflict => 400,
            ErrorCode::InsufficientRank => 403,
            ErrorCode::InsufficientRankToManageUser => 403,
            ErrorCode::CannotModifyDefaultRole => 400,
            ErrorCode::DmThreadMissingRecipients => 400,
            ErrorCode::DmThreadSinglePersonOnly => 400,
            ErrorCode::GdmThreadMissingRecipients => 400,
            ErrorCode::GdmTooManyMembers => 400,
            ErrorCode::GdmRequiresFriend => 403,
            ErrorCode::DmGdmOnlyOutsideRoom => 400,
            ErrorCode::BitrateTooHigh => 400,
            ErrorCode::CannotSetBitrateForNonVoiceThread => 400,
            ErrorCode::CannotSetUserLimitForNonVoiceThread => 400,
            ErrorCode::OnlyGdmCanHaveIcons => 400,
            ErrorCode::MediaNotAnImage => 422,
            ErrorCode::InvalidParentChannelType => 400,
            ErrorCode::OwnerIdCannotBeChanged => 400,
            ErrorCode::ChannelDoesntHaveText => 400,
            ErrorCode::ChannelDoesntHaveVoice => 400,
            ErrorCode::CannotEditThreadMemberList => 400,
            ErrorCode::InvalidThreadType => 400,
            ErrorCode::CantDeleteThatMessage => 403,
            ErrorCode::CantDeleteThatMessageType => 403,
            ErrorCode::CannotDeleteLatestMessageVersion => 400,
            ErrorCode::MaxPinsReached => 400,
            ErrorCode::OnlyGdmCanUpgrade => 400,
            ErrorCode::NotThreadOwner => 403,
            ErrorCode::ThreadAlreadyInRoom => 400,
            ErrorCode::GuestsCannotJoinPublicRooms => 403,
            ErrorCode::YouAreBanned => 403,
            ErrorCode::CantAddThatUser => 400,
            ErrorCode::OnlyBotsCanUseThis => 403,
            ErrorCode::BotIsNotABridge => 403,
            ErrorCode::NotPuppetOwner => 403,
            ErrorCode::CannotAddRoleAboveYourRole => 403,
            ErrorCode::CannotRemoveRoleAboveYourRole => 403,
            ErrorCode::NotRoomOwner => 403,
            ErrorCode::RoomOwnerMustHaveMfa => 403,
            ErrorCode::CannotKickFromServerRoom => 400,
            ErrorCode::RoomOwnerCannotLeave => 400,
            ErrorCode::CannotBanRoomOwner => 403,
            ErrorCode::CannotAddRolesToYourself => 403,
            ErrorCode::UserIsNotAGuestAccount => 400,
            ErrorCode::AddAuthMethodFirst => 400,
            ErrorCode::CannotCreateInviteForServerRoom => 400,
            ErrorCode::CannotAddRolesToInvite => 400,
            ErrorCode::ChannelNotInRoomOrGdm => 400,
            ErrorCode::ChannelNotInRoom => 400,
            ErrorCode::GuestsCannotCreateServerInvites => 403,
            ErrorCode::GuestsCannotListServerInvites => 403,
            ErrorCode::UnknownResponseType => 400,
            ErrorCode::BadRedirectUri => 400,
            ErrorCode::InvalidScope => 400,
            ErrorCode::NoRedirectUriConfigured => 400,
            ErrorCode::InvalidClientId => 400,
            ErrorCode::MissingCode => 400,
            ErrorCode::MissingRedirectUri => 400,
            ErrorCode::MissingCodeVerifier => 400,
            ErrorCode::UnsupportedCodeChallengeMethod => 400,
            ErrorCode::MissingRefreshToken => 400,
            ErrorCode::UnsupportedGrantType => 400,
            ErrorCode::NotAnOauthToken => 400,
            ErrorCode::CanOnlyCreateUserOnOwnServer => 403,
            ErrorCode::CanOnlySyncForThisServer => 403,
            ErrorCode::PlatformNameRequiredForBridge => 400,
            ErrorCode::CantCreateThatUser => 400,
            ErrorCode::FieldMissingName => 400,
            ErrorCode::UnknownField => 400,
            ErrorCode::NoData => 400,
            ErrorCode::ChannelIsNotACalendar => 400,
            ErrorCode::CannotRsvpOtherPeople => 403,
            ErrorCode::WebhookNotInRoom => 400,
            ErrorCode::CannotSetPermissionsOnThisChannelType => 400,
            ErrorCode::CannotSetPermissionsOnParentChannelOfType => 400,
            ErrorCode::CannotRemoveLastAuthMethod => 400,
            ErrorCode::CannotDmThisUser => 403,
            ErrorCode::DmsNotAllowedFromThisUser => 403,
            ErrorCode::BotsCannotUseThisEndpoint => 403,
            ErrorCode::ChannelDoesNotSupportTags => 400,
            ErrorCode::FailedToEncodeMetrics => 500,
            ErrorCode::CannotMoveToDifferentRoom => 400,
            ErrorCode::CannotMoveToThreadWithoutVoice => 400,
            ErrorCode::NotConnectedToAnyThread => 400,
            ErrorCode::CannotMoveToThreadInDifferentRoom => 400,
            ErrorCode::CannotCloseDefaultBranch => 400,
            ErrorCode::CannotMergeDefaultBranch => 400,
            ErrorCode::BranchHasNoParent => 400,
            ErrorCode::CannotTagAnotherTag => 400,
            ErrorCode::CannotFriendThisUser => 403,
            ErrorCode::FriendRequestsPaused => 400,
            ErrorCode::UnblockUserFirst => 403,
            ErrorCode::SudoSessionExpired => 401,
            ErrorCode::InvalidOrExpiredCode => 400,
            ErrorCode::TotpNotInitialized => 400,
            ErrorCode::TotpAlreadyEnabled => 400,
            ErrorCode::InvalidTotpCode => 400,
            ErrorCode::TotpNotEnabled => 400,
            ErrorCode::NotBotOwner => 403,
            ErrorCode::UserIsNotABot => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Ratelimit => 429,
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
