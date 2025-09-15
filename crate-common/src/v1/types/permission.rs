use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::util::deserialize_sorted;

pub mod defaults;

// should i rename Admin to RoomAdmin? it might be confusing to have ThreadAdmin
// be a different permission though
// should i split out room, thread, user, and server permissions? yeah.
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

    /// can add bots, configure bots they have added, and kick bots they have added
    // TODO: deprecate/merge with BotsManage
    BotsAdd,

    /// can configure all bots and kick all bots
    /// implies BotsAdd
    // TODO: rename to IntegrationsManage? or Tinker
    BotsManage,

    /// can add emoji and remove emoji they have added
    // TODO: merge with EmojiManage
    EmojiAdd,

    /// can remove all emoji
    /// implies EmojiAdd
    EmojiManage,

    /// can use custom emoji not added to this room
    EmojiUseExternal,

    /// create invites, view metadata for invites they created, and delete invites they created
    InviteCreate,

    /// view metadata for all invites and delete all invites
    /// implies InviteCreate
    InviteManage,

    /// (unimplemented) ban members and unban members they have banned
    MemberBan,

    /// (unimplemented) unban any member
    /// implies MemberBan
    // TODO: remove, not worth it
    MemberBanManage,

    /// allow adding users with type Puppet and use timestamp massaging
    /// intended for bridge bots
    // TODO: rename to `Bridge`?
    MemberBridge,

    /// kick members
    MemberKick,

    /// edit member name
    // TODO: rename to MemberManageProfile
    // TODO: add MemberChangeProfile for changing your own nickname/profile
    MemberManage,

    /// send attachments
    /// requires MessageCreate
    MessageAttachments,

    /// send messages
    MessageCreate,

    /// delete (and TODO: undelete) other people's messages; undelete is not
    /// possible if the message was deleted by its creator (you can only recover
    /// messages deleted by other moderators)
    MessageDelete,

    /// (internal) can edit this message
    /// requires MessageCreate
    // TODO: remove (for now?)
    MessageEdit,

    /// send embeds (link previews)
    /// requires MessageCreate
    MessageEmbeds,

    /// (unimplemented) mention @room, @thread, and all roles
    /// requires MessageCreate
    MessageMassMention,

    /// (unimplemented) move messages between threads
    MessageMove,

    /// (unimplemented) pin (and unpin) messages
    MessagePin,

    /// (unimplemented) use custom avatar (otherwise use default avatar)
    // TODO: merge with MemberChangeProfile
    ProfileAvatar,

    /// (unimplemented) use a custom name (nickname), description, etc
    // TODO: merge with MemberChangeProfile
    ProfileOverride,

    /// add new reactions
    // TODO: can still react with existing reactions
    ReactionAdd,

    /// remove all reactions
    // TODO: rename to ReactionPurge
    ReactionClear,

    /// add and remove roles from members
    RoleApply,

    /// create, edit, and delete roles. also managing permissions in general.
    RoleManage,

    /// edit name, description, really anything else
    RoomManage,

    /// (server) the "root user" permission that allows everything.
    /// probably shouldn't implement this for the same reasons as Admin
    /// but i think it is a necessary evil
    // TODO: remove, Admin works fine here
    ServerAdmin,

    /// (server, unimplemented) can access metrics (prometheus)
    ServerMetrics,

    /// (server, unimplemented) can view everything
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

    /// (unimplemented) can create document threads
    // TODO: remove for now, will re-add when documents are implemented
    ThreadCreateDocument,

    /// (unimplemented) can create event threads
    // TODO: remove for now, will re-add when events are implemented
    ThreadCreateEvent,

    /// (unimplemented) can create forum (linear) threads
    // TODO: rename to ThreadCreateForum
    ThreadCreateForumLinear,

    /// (unimplemented) can create forum (tree) threads
    // TODO: remove
    ThreadCreateForumTree,

    /// (unimplemented) can create private threads (what is "private"?)
    ThreadCreatePrivate,

    /// (unimplemented) can create public threads (what is "public"?)
    ThreadCreatePublic,

    /// (unimplemented) can create table threads
    // TODO: remove for now, will re-add when tables are implemented
    ThreadCreateTable,

    /// (unimplemented) can create voice threads
    ThreadCreateVoice,

    // TODO: add permissions for category threads when they are implemented
    // TODO: rename to ThreadRemove
    /// delete (and undelete) threads
    ThreadDelete,

    /// change name/description of threads
    // TODO: possibly rename to ThreadManage
    ThreadEdit,

    /// (unimplemented) move threads across rooms
    /// this could be a pretty tricky permission to get right...
    /// this isnt the same as email forwarding
    ThreadForward,

    /// lock (and unlock) threads
    ThreadLock,

    /// (unimplemented) pin (and unpin) threads
    ThreadPin,

    /// (unimplemented) create announcements
    /// requires ThreadCreate*
    // rename to ThreadCreateAnnouncement?
    ThreadPublish,

    // TODO: remove user permissions for now, they aren't used anywhere
    /// (user) access dms
    UserDms,

    /// (user) edit profile (name, description, etc)
    UserProfile,

    /// (user) manage sessions
    UserSessions,

    /// (user) set status
    UserStatus,

    /// (internal) can view this thing; see other ViewFoo permissions for things you can set
    // TODO: make this not internal; ie let people restrict who can view what
    View,

    /// view audit log
    ViewAuditLog,

    /// connect and listen to voice threads
    VoiceConnect,

    /// (unimplemented) stop someone from listening
    VoiceDeafen,

    /// disconnect members from voice threads
    VoiceDisconnect,

    /// (unimplemented) move members between voice threads
    VoiceMove,

    /// (unimplemented) stop someone from talking
    VoiceMute,

    /// (unimplemented) talk louder
    /// requires VoiceSpeak
    VoicePriority,

    /// (unimplemented) talk in voice threads
    /// requires VoiceConnect
    VoiceSpeak,

    /// (unimplemented) stream video and screenshare in voice threads
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
