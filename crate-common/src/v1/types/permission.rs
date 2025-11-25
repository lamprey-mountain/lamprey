#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::util::deserialize_sorted;

pub mod defaults;

/// a permission that lets a user do something
#[derive(
    Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, strum::EnumIter, strum::EnumCount,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Permission {
    /// Allows **everything**
    /// probably a major footgun. i'd like to remove it, but theres legit purposes for it right now...
    Admin,

    /// can configure all bots and kick all bots
    IntegrationsManage,

    /// can add and remove emoji
    EmojiManage,

    /// can use custom emoji not added to this room
    EmojiUseExternal,

    /// create invites, view metadata for invites they created, and delete invites they created
    InviteCreate,

    /// view metadata for all invites and delete all invites
    /// implies InviteCreate
    InviteManage,

    /// ban and unban members
    MemberBan,

    /// allow adding users with type Puppet and use timestamp massaging
    /// intended for bridge bots
    // TODO: rename to `Bridge`
    MemberBridge,

    /// kick members
    MemberKick,

    /// edit members' nicknames
    MemberNicknameManage,

    /// send attachments
    /// requires MessageCreate
    MessageAttachments,

    /// send messages
    MessageCreate,

    /// delete other people's messages
    MessageDelete,

    /// remove and restore messages
    MessageRemove,

    /// send embeds (link previews)
    /// requires MessageCreate
    MessageEmbeds,

    /// (unimplemented) mention @everyone, @here, and all roles
    /// requires MessageCreate
    MessageMassMention,

    /// (unimplemented) move messages between channels
    MessageMove,

    /// pin and unpin messages
    MessagePin,

    /// use a custom nickname
    MemberNickname,

    /// timeout members
    MemberTimeout,

    /// add new reactions
    // TODO: can still react with existing reactions
    ReactionAdd,

    /// remove all reactions
    ReactionPurge,

    /// add and remove roles from members
    RoleApply,

    /// create, edit, and delete roles. also managing permissions in general.
    RoleManage,

    /// edit name, description, really anything else
    RoomManage,

    /// (server, unimplemented) can access metrics (prometheus)
    ServerMetrics,

    /// (server) can view the server room and all members on the server
    ServerOversee,

    /// (server, unimplemented) access reports
    ServerReports,

    /// (unimplemented) apply tags to threads
    /// applying tags to rooms would probably be a RoomEdit thing
    // TODO: merge with ThreadEdit?
    TagApply,

    /// (unimplemented) create and delete tags
    // TODO: merge with ChannelManage or ChannelEdit?
    TagManage,

    /// can change channel names and topics
    ChannelEdit,

    /// can create, remove, and archive channels. can also list all channels.
    ChannelManage,

    /// can create private threads
    ThreadCreatePrivate,

    /// can create public threads
    ThreadCreatePublic,

    /// remove and archive threads, and move threads between channels. can also view all threads.
    ThreadManage,

    /// change name and description of threads
    ThreadEdit,

    /// lock and unlock threads
    // TODO: merge with ThreadManage?
    ThreadLock,

    /// Can view channels
    ViewChannel,

    /// view audit log
    ViewAuditLog,

    // TODO
    // /// view room analytics
    // ViewAnalytics,
    /// connect and listen to voice threads
    VoiceConnect,

    /// stop someone from listening
    VoiceDeafen,

    /// disconnect members from voice threads
    VoiceDisconnect,

    /// move members between voice threads
    VoiceMove,

    /// stop someone from talking
    VoiceMute,

    /// talk louder
    /// requires VoiceSpeak
    VoicePriority,

    /// talk in voice threads
    /// requires VoiceConnect
    VoiceSpeak,

    /// stream video and screenshare in voice threads
    /// requires VoiceConnect
    VoiceVideo,

    /// can manage calendar events
    CalendarEventManage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwrites {
    #[serde(flatten)]
    inner: Vec<PermissionOverwrite>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwrite {
    /// id of role or user
    pub id: Uuid,

    /// whether this is for a user or role
    #[serde(rename = "type")]
    pub ty: PermissionOverwriteType,

    /// extra permissions allowed here
    #[serde(deserialize_with = "deserialize_sorted")]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[serde(deserialize_with = "deserialize_sorted")]
    pub deny: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwriteSet {
    /// whether this is for a user or role
    #[serde(rename = "type")]
    pub ty: PermissionOverwriteType,

    /// extra permissions allowed here
    #[serde(deserialize_with = "deserialize_sorted")]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[serde(deserialize_with = "deserialize_sorted")]
    pub deny: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum PermissionOverwriteType {
    /// permission overrides for a role
    Role,

    /// permission overrides for a user
    User,
}
