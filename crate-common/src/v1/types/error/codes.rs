//! error codes

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

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

    // TODO: split this into
    // - "missing session"
    // - "missing user"
    // - "this is a federated endpoint and requires server authentication"
    /// missing authentication
    #[error("missing authentication")]
    MissingAuth,

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

    /// unknown harvest
    #[error("unknown harvest")]
    UnknownHarvest,

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

    /// unknown voice state
    #[error("unknown voice state")]
    UnknownVoiceState,

    /// unknown call
    #[error("unknown call")]
    UnknownCall,

    /// unknown sfu
    #[error("unknown sfu")]
    UnknownSfu,

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

    /// invites are paused
    #[error("invites are paused")]
    InvitesPaused,

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

    /// cannot manage remote user
    #[error("cannot manage remote user")]
    CannotManageRemoteUser,

    /// script error
    #[error("script error")]
    ScriptError,

    /// this room type doesnt have channels
    #[error("room_type_no_channels")]
    RoomTypeNoChannels,

    /// interaction not allowed
    #[error("interaction not allowed")]
    InteractionNotAllowed,
}
