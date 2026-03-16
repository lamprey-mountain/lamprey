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
    /// Allows **everything**. Bypasses all locks, overwrites, etc. People with
    /// this permission effectively become a second owner.
    Admin,

    /// can add, configure, and kick bots
    IntegrationsManage,

    /// for bridge bots, enables bridging features
    ///
    /// - can add users with type Puppet
    /// - can use timestamp massaging
    IntegrationsBridge,

    /// can add and remove emoji
    EmojiManage,

    /// can use custom emoji not added to this room
    EmojiUseExternal,

    /// create invites, view metadata for invites they created, and delete invites they created
    InviteCreate,

    /// view metadata for all invites and delete all invites
    InviteManage,

    /// ban and unban members
    MemberBan,

    /// kick members
    MemberKick,

    /// edit members' nicknames
    MemberNicknameManage,

    /// use a custom nickname
    MemberNickname,

    /// timeout members
    MemberTimeout,

    /// send attachments
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
    MessageEmbeds,

    /// mention @everyone, @here, and all roles
    MessageMassMention,

    /// (unimplemented) move messages between channels
    MessageMove,

    /// pin and unpin messages
    MessagePin,

    /// add new reactions
    ReactionAdd,

    /// remove reactions
    ReactionManage,

    /// add and remove roles from members.
    RoleApply,

    /// create, edit, and delete roles. add and remove overwrites for channels.
    RoleManage,

    /// edit name, description, really anything else
    RoomEdit,

    /// (server, unimplemented) can access metrics (prometheus)
    ServerMetrics,

    /// (server) can perform server maintenance tasks
    ///
    /// for example, they can:
    ///
    /// - reindex search indexes
    /// - setup and stop voice sfus
    /// - garbage collect
    ServerMaintenance,

    /// (server) can view the server room and all members on the server
    ///
    /// this should be added to all "server moderator/admin/operator" roles
    ServerOversee,

    /// unaffected by slowmode
    ChannelSlowmodeBypass,

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
    ThreadManage,

    /// change name and description of threads
    ThreadEdit,

    /// Can view channels
    ChannelView,

    /// view audit log
    AuditLogView,

    /// view room analytics
    AnalyticsView,

    /// stop someone from listening
    VoiceDeafen,

    /// disconnect members from voice threads, move members between voice channels
    VoiceMove,

    /// stop someone from talking
    VoiceMute,

    /// talk louder
    VoicePriority,

    /// talk in voice threads
    VoiceSpeak,

    /// stream video and screenshare in voice threads
    VoiceVideo,

    /// use voice activity detection
    VoiceVad,

    /// can request to speak in broadcast channels
    VoiceRequest,

    /// can broadcast voice to all channels in a category
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
    RoomManage,

    /// can create, edit, and delete users. can view all users.
    UserManage,

    /// can disable or delete their own account
    UserManageSelf,

    /// can edit their own profile
    UserProfileSelf,

    /// can create new applications
    ApplicationCreate,

    /// can edit and delete all applications. can list all applications on the server.
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
    RoomJoinForce,
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
        matches!(
            self,
            Permission::ServerMetrics
                | Permission::ServerMaintenance
                | Permission::ServerOversee
                | Permission::RoomCreate
                | Permission::RoomManage
                | Permission::UserManage
                | Permission::UserManageSelf
                | Permission::UserProfileSelf
                | Permission::ApplicationCreate
                | Permission::ApplicationManage
                | Permission::DmCreate
                | Permission::FriendCreate
                | Permission::RoomJoin
                | Permission::RoomJoinForce
        )
    }

    /// if this is a room permission
    ///
    /// these can only be set at the top level (ie. not as channel overwrites)
    pub fn is_room(&self) -> bool {
        todo!()
    }
}
