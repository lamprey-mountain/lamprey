#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
use uuid::Uuid;

#[cfg(feature = "serde")]
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

    /// can add, configure, and kick bots
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

    /// use a custom nickname
    MemberNickname,

    /// timeout members
    MemberTimeout,

    /// send attachments
    /// requires MessageCreate
    MessageAttachments,

    /// send messages
    MessageCreate,

    /// can send messages in threads
    ///
    /// in threads, this must be used instead of MessageCreate.
    MessageCreateThread,

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

    /// add new reactions
    // TODO: can still react with existing reactions
    ReactionAdd,

    /// remove all reactions
    // TODO: rename to ReactionManage
    ReactionPurge,

    /// add and remove roles from members
    RoleApply,

    /// create, edit, and delete roles. also managing permissions in general.
    RoleManage,

    /// edit name, description, really anything else
    // TODO: maybe rename this to prevent conflict with RoomManageServer
    RoomManage,

    /// (server, unimplemented) can access metrics (prometheus)
    // TODO: enforce this permission
    ServerMetrics,

    /// (server) can perform server maintenance tasks
    ServerMaintenance,

    /// (server) can view the server room and all members on the server
    ServerOversee,

    /// (server, unimplemented) access reports
    // TODO: remove
    ServerReports,

    /// (unimplemented) apply tags to threads
    /// applying tags to rooms would probably be a RoomEdit thing
    // TODO: merge with ThreadEdit?
    TagApply,

    /// (unimplemented) create and delete tags
    // TODO: merge with ChannelManage or ChannelEdit?
    TagManage,

    /// unaffected by slowmode
    BypassSlowmode,

    /// can change channel names and topics
    ChannelEdit,

    /// can create, remove, and archive channels. can also list all channels.
    ChannelManage,

    /// can create private threads
    ThreadCreatePrivate,

    /// can create public threads
    ThreadCreatePublic,

    /// can do moderation actions on threads
    ///
    /// - remove and archive threads
    /// - move threads between channels
    /// - view all private threads
    /// - manage document branches
    // TODO: split apart permissions
    ThreadManage,

    /// change name and description of threads
    ThreadEdit,

    /// lock and unlock threads
    // TODO: merge with ThreadManage?
    // TODO: make this allow locking/unlocking entire channels?
    // TODO: MemberTimeout is also somewhat relevant for this permission?
    ThreadLock,

    /// Can view channels
    ViewChannel,

    /// view audit log
    ViewAuditLog,

    /// view room analytics
    ViewAnalytics,

    /// connect and listen to voice threads
    // TODO: remove
    VoiceConnect,

    /// stop someone from listening
    VoiceDeafen,

    /// disconnect members from voice threads
    // TODO: maybe merge with VoiceMove?
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

    /// use voice activity detection (TODO)
    VoiceVad,

    /// can request to speak in broadcast channels
    VoiceRequest,

    /// can broadcast voice to all channels in a category (TODO)
    VoiceBroadcast,

    /// can create calendar events and delete their own calendar events
    CalendarEventCreate,

    /// can rsvp to calendar events
    CalendarEventRsvp,

    /// can manage calendar events
    CalendarEventManage,

    /// can create, edit, and remove their own documents in wiki channels.
    DocumentCreate,

    /// can edit documents, including documents outside of wikis.
    DocumentEdit,

    /// can comment on documents, including documents outside of wikis.
    DocumentComment,

    /// can create new rooms.
    RoomCreate,

    /// can delete and quarantine rooms, and view all rooms, room templates, dms, and gdms.
    // TODO: maybe rename this to prevent conflict with RoomManage
    RoomManageServer,

    /// can create, edit, and delete users. can view all users.
    // TODO: maybe deduplicate this with ServerOversee?
    UserManage,

    /// can disable or delete their own account
    UserDeleteSelf,

    /// can edit their own profile
    UserProfile,

    /// can create new applications
    ApplicationCreate,

    /// can edit and delete all applications. can list all applications on the server.
    // TODO: implement
    ApplicationManage,

    /// can create new dms and gdms
    DmCreate,

    /// can send friend requests
    FriendCreate,

    /// can manually join and leave rooms and gdms (use invites)
    RoomJoin,

    /// set call metadata (ie. the topic)
    ///
    /// requires the ability to speak: not muted, not suppressed, has VoiceSpeak.
    CallUpdate,

    /// can forcibly make other users join and leave rooms and gdms. can join any room and gdm.
    // TODO: implement
    RoomForceJoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwrites {
    #[cfg_attr(feature = "serde", serde(flatten))]
    inner: Vec<PermissionOverwrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwrite {
    /// id of role or user
    pub id: Uuid,

    /// whether this is for a user or role
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: PermissionOverwriteType,

    /// extra permissions allowed here
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_sorted"))]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_sorted"))]
    pub deny: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverwriteSet {
    /// whether this is for a user or role
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: PermissionOverwriteType,

    /// extra permissions allowed here
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_sorted"))]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_sorted"))]
    pub deny: Vec<Permission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum PermissionOverwriteType {
    /// permission overrides for a role
    Role,

    /// permission overrides for a user
    User,
}

impl Permission {
    /// if this permission is applicable to webhooks
    // TODO(#898): permissions for webhooks
    pub fn is_webhook_usable(&self) -> bool {
        matches!(
            self,
            Permission::MessageMassMention
                | Permission::EmojiUseExternal
                | Permission::MessageAttachments
                | Permission::MessageEmbeds
        )
    }

    /// if this is a server permission
    ///
    /// these can only be set in the server room
    pub fn is_server(&self) -> bool {
        todo!()
    }

    /// if this is a room permission
    ///
    /// these can only be set at the top level (ie. not as channel overwrites)
    pub fn is_room(&self) -> bool {
        todo!()
    }
}
