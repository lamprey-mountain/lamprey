use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{util::deserialize_sorted_permissions, RoleId, UserId};

pub mod defaults;

// should i rename Admin to RoomAdmin? it might be confusing to have ThreadAdmin
// be a different permission though
// should i split out room, thread, user, and server permissions?
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
    BotsAdd,

    /// can configure all bots and kick all bots
    /// implies BotsAdd
    BotsManage,

    /// can add emoji and remove emoji they have added
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
    MemberBanManage,

    /// (unimplemented) allow adding users with type Puppet (and use timestamp massaging if/when implemented)
    /// intended for bridge bots
    MemberBridge,

    /// kick members
    MemberKick,

    /// edit member name
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
    ProfileAvatar,

    /// (unimplemented) use a custom name (nickname), description, etc
    ProfileOverride,

    /// (unimplemented) add new reactions (can still react with existing reactions)
    ReactionAdd,

    /// (unimplemented) remove reactions
    ReactionClear,

    /// add and remove roles from members
    RoleApply,

    /// create, edit, and delete roles
    RoleManage,

    /// edit name, description, really anything else
    RoomManage,

    /// (server) the "root user" permission that allows everything.
    /// probably shouldn't implement this for the same reasons as Admin
    /// but i think it is a necessary evil
    ServerAdmin,

    /// (server) can access metrics (prometheus)
    ServerMetrics,

    /// (server) can view everything
    ServerOversee,

    /// (server) access reports
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
    ThreadCreateDocument,

    /// (unimplemented) can create event threads
    ThreadCreateEvent,

    /// (unimplemented) can create forum (linear) threads
    ThreadCreateForumLinear,

    /// (unimplemented) can create forum (tree) threads
    ThreadCreateForumTree,

    /// (unimplemented) can create private threads (what is "private"?)
    ThreadCreatePrivate,

    /// (unimplemented) can create public threads (what is "public"?)
    ThreadCreatePublic,

    /// (unimplemented) can create table threads
    ThreadCreateTable,

    /// (unimplemented) can create voice threads
    ThreadCreateVoice,

    /// delete (and undelete) threads
    ThreadDelete,

    /// change name/description of threads
    ThreadEdit,

    /// (unimplemented) move threads across rooms
    /// this could be a pretty tricky permission to get right...
    /// this isnt the same as email forwarding
    ThreadForward,

    /// (unimplemented) lock (and unlock) threads
    ThreadLock,

    /// (unimplemented) pin (and unpin) threads
    ThreadPin,

    /// (unimplemented) create announcements
    /// requires ThreadCreate*
    ThreadPublish,

    /// (user) access dms
    UserDms,

    /// (user) edit profile (name, description, etc)
    UserProfile,

    /// (user) manage sessions
    UserSessions,

    /// (user) set status
    UserStatus,

    /// (internal) can view this thing; see other ViewFoo permissions for things you can set
    View,

    /// view audit log
    ViewAuditLog,

    /// (unimplemented) connect and listen to voice threads
    VoiceConnect,

    /// (unimplemented) stop someone from listening
    VoiceDeafen,

    /// (unimplemented) disconnect members from voice threads
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

// other potential permissions
//
// - ViewHistory: doesn't make sense here. can see all {messages,threads} {after
//   they join,while they're online}? maybe it could make all previous threads
//   private by default and require mentions/manual adding
// - ViewAnalytics: view some sort of analytics. that would require me to add
//   some sort of analytics thing which im not sure i want.
// - VoiceVAD: seems like it should be a user setting? and if people keep
//   turning it on, its probably a moderation problem?
// - RoomOwner: let rooms have one(?) owner who has full control over
//   everything. this is a good way to prevent softlocking by ensuring at least
//   one person has full permissions.
// - VoiceRequest: request to speak. VoiceSpeak and VoiceVideo allow override for that person in that thread
//   could be useful, but maybe later
// - WebhooksManage: this can be done with Bot* permissions
// - EventManage: manage document threads
// - EventRsvp: exactly what it sounds like
// - EventRsvpManage: exactly what it sounds like
// - DocumentEdit: exactly what it sounds like
// - DocumentManage: manage event threads
// - MessagePoll: needs impl
// - MessageForms: needs impl; see guilded
// - MessageInteractions: needs impl; for bots
// - MessageMasquerade: set custom names/avatars similar to webhooks on other
//   platforms. puppets exist, so this might not be necessary? this is certainly
//   more convenient though.
// - InteractionFoo: alternative to MessageInteractions
// - ThreadAssign: assign a thread to someone
// - Reports: needs impl; can access the reporting system; copies thread perms (eg. ThreadArchive for closing reports)
// - SlowmodeBypassThread: unsure about slowmode in general. leaning towards
//   "will add", but idk it feels like another inelegant moderation hack
// - SlowmodeBypassMessage: see above
// - ThreadList: see ViewHistory; if disabled, threads are unlisted by default?
//   (eg. you can only view joined threads). kind of an interesting idea.
// - RoomManage: might want to split out safer perms from more dangerous
//   ones, eg. changing name/topic vs changing visibility. i'll probably dump
//   everything into RoomEdit for now, until i hear of a use case.
// - MessageEmail: sending messages by email?
// - MessageTodos: check/uncheck checkboxes in messages
//
// would i rename ThreadCreateFoo to FooCreate? maybe!
// also, i don't want it to become too complicated or have too many perms!

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverrides {
    #[serde(flatten)]
    inner: Vec<PermissionOverrideWithTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverride {
    /// extra permissions allowed here
    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub deny: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum PermissionOverridable {
    /// permission overrides for a role
    Role { role_id: RoleId },

    /// permission overrides for a user
    User { user_id: UserId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverrideWithTarget {
    #[serde(flatten)]
    pub target: PermissionOverridable,

    #[serde(flatten)]
    pub perms: PermissionOverride,
}
