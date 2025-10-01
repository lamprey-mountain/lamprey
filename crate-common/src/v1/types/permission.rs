use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::util::deserialize_sorted;

pub mod defaults;

/// a permission that lets a user do something
///
/// - unimplemented: the feature this permission refers to does not yet exist
/// - internal: this is calculated by the server and cannot be manually added
/// - user: this is a permission granted to user sessions/bots, not threads/rooms
/// - server: this is a permission granted to server tokens
///
/// thread permissions are combined with and (you need both permissions)
#[derive(
    Debug,
    Hash,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    strum::EnumIter,
    strum::EnumCount,
)]
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
    // TODO: rename to `Bridge`?
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

    /// (unimplemented) mention @room, @thread, and all roles
    /// requires MessageCreate
    MessageMassMention,

    /// (unimplemented) move messages between threads
    MessageMove,

    /// pin and unpin messages
    MessagePin,

    /// use a custom nickname
    MemberNickname,

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
    TagApply,

    /// (unimplemented) create and delete tags
    TagManage,

    /// archive (and unarchive) threads
    ThreadArchive,

    /// (unimplemented) can create chat threads
    ThreadCreateChat,

    /// (unimplemented) can create forum threads
    ThreadCreateForum,

    /// (unimplemented) can create private threads (what is "private"?)
    ThreadCreatePrivate,

    /// (unimplemented) can create public threads (what is "public"?)
    ThreadCreatePublic,

    /// can create voice threads
    ThreadCreateVoice,

    /// remove (and restore) threads
    ThreadRemove,

    /// change name and description of threads
    ThreadEdit,

    /// (unimplemented) move threads across rooms
    /// this could be a pretty tricky permission to get right...
    /// this isnt the same as email forwarding
    ThreadForward,

    /// lock and unlock threads
    ThreadLock,

    /// reorder and pin threads
    ThreadManage,

    /// (unimplemented) create announcements
    /// requires ThreadCreate*
    ThreadPublish,

    /// (internal) can view this thing; see other ViewFoo permissions for things you can set
    // TODO: make this not internal; ie let people restrict who can view what
    //
    // steps:
    // - remove View
    // - remove ensure_view
    // - enforce current view logic in perms.for_{room_thread}
    // - add ViewThread (view all threads/view this room)
    View,

    /// view audit log
    ViewAuditLog,

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
