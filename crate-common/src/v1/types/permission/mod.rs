use lamprey_macros::record;
use thiserror::Error;

use crate::v1::types::PermissionOverwriteId;
#[cfg(feature = "serde")]
use crate::v1::types::util::deserialize_sorted;

pub mod defaults;

/// an error that occurred whilst trying to convert a `u32` into a `Permission`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid permission discriminant: {}", self.0)]
pub struct PermissionConversionError(pub u32);

macro_rules! define_permissions {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident
        ),* $(,)?
    ) => {
        /// a permission that lets a user do something
        #[record]
        #[derive(Hash, Copy, PartialEq, Eq, PartialOrd, Ord, strum::EnumIter, strum::EnumCount)]
        pub enum Permission {
            $(
                $(#[$meta])*
                $variant,
            )*
        }

        bitflags::bitflags! {
            /// compact bitset representation of permissions
            ///
            /// represented with a single `u128`
            #[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
            pub struct PermissionBits: u128 {
                $(
                    const $variant = 1 << (Permission::$variant as u32);
                )*

                /// permissions that affect one's ability to view something
                const VIEW_PERMS = Self::ChannelView.bits() | Self::AuditLogView.bits() | Self::AnalyticsView.bits();

                /// permissions for lurkers in broadcast channels
                const BROADCAST_LURKER_PERMS = Self::VIEW_PERMS.bits() | Self::VoiceRequest.bits() | Self::VoiceVad.bits();

                /// permissions for quarantined users
                ///
                /// this includes view perms + nickname
                const QUARANTINE_PERMS = Self::VIEW_PERMS.bits() | Self::MemberNickname.bits();

                /// bitset with **every** permission (including ones that dont exist yet)
                const EVERYTHING = u128::MAX;
            }
        }

        impl TryFrom<u32> for Permission {
            type Error = PermissionConversionError;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                $(
                    if value == Permission::$variant as u32 {
                        return Ok(Permission::$variant);
                    }
                )*
                Err(PermissionConversionError(value))
            }
        }
    };
}

define_permissions! {
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

    /// (server) can access metrics (prometheus)
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

    /// can manage rooms on this server
    ///
    /// - delete and quarantine rooms
    /// - enable and disable room features
    /// - view all rooms and room templates
    /// - view all dms and gdms (temp?)
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

    /// can create, edit, and delete redexes. can also stop evals.
    // TODO: rename to RedexManage
    ScriptManage,

    /// can view redex logs, traces, metrics, and other debugging info
    // TODO: rename to RedexInspect
    ScriptInspect,
    // TODO: maybe add new EmojiCreate permission
    // like discord's expression create permission
}

// TODO: add VoiceWhisper
// /// whisper to other people
// VoiceWhisper,

impl From<Permission> for PermissionBits {
    fn from(p: Permission) -> Self {
        PermissionBits::from_bits_retain(1 << (p as u32))
    }
}

impl From<&Permission> for PermissionBits {
    fn from(p: &Permission) -> Self {
        PermissionBits::from(*p)
    }
}

impl From<Vec<Permission>> for PermissionBits {
    fn from(perms: Vec<Permission>) -> Self {
        perms.into_iter().map(PermissionBits::from).collect()
    }
}

impl From<&[Permission]> for PermissionBits {
    fn from(perms: &[Permission]) -> Self {
        perms.iter().copied().map(PermissionBits::from).collect()
    }
}

impl FromIterator<Permission> for PermissionBits {
    fn from_iter<I: IntoIterator<Item = Permission>>(iter: I) -> Self {
        iter.into_iter()
            .fold(PermissionBits::empty(), |mut acc, p| {
                acc.insert(p.into());
                acc
            })
    }
}

impl From<PermissionBits> for Vec<Permission> {
    fn from(bits: PermissionBits) -> Self {
        use strum::IntoEnumIterator;
        Permission::iter()
            .filter(|&p| bits.contains(p.into()))
            .collect()
    }
}

impl PermissionBits {
    /// Check if a specific permission is set
    pub fn has(self, permission: Permission) -> bool {
        self.contains(permission.into())
    }

    /// Add a permission
    pub fn add(&mut self, permission: Permission) {
        self.insert(permission.into());
    }

    /// Remove a permission
    pub fn remove_perm(&mut self, permission: Permission) {
        self.remove(permission.into());
    }

    /// Add all permissions from another PermissionBits
    pub fn add_all(&mut self, other: PermissionBits) {
        self.insert(other);
    }

    /// Remove all permissions that are set in another PermissionBits
    pub fn remove_all(&mut self, other: PermissionBits) {
        self.remove(other);
    }

    /// Create a PermissionBits from a slice of Permissions
    pub fn from_slice(perms: &[Permission]) -> Self {
        perms.iter().copied().map(PermissionBits::from).collect()
    }

    /// remove all permissions except those in the allowed set
    pub fn mask(&mut self, mask: PermissionBits) {
        *self &= mask;
    }

    /// Check if any of the given permissions are set
    pub fn has_any(self, perms: &[Permission]) -> bool {
        self.intersects(Self::from_slice(perms))
    }

    /// Check if all of the given permissions are set
    pub fn has_all(self, perms: &[Permission]) -> bool {
        self.contains(Self::from_slice(perms))
    }

    /// Get all permissions contained in this PermissionBits
    pub fn to_vec(self) -> Vec<Permission> {
        use strum::IntoEnumIterator;
        Permission::iter()
            .filter(|&p| self.contains(p.into()))
            .collect()
    }
}

// TODO: either use or remove this
#[record]
#[derive(PartialEq, Eq)]
pub struct PermissionOverwrites {
    #[cfg_attr(feature = "serde", serde(flatten))]
    inner: Vec<PermissionOverwrite>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct PermissionOverwrite {
    /// id of role or user
    pub id: PermissionOverwriteId,

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

#[record]
#[derive(PartialEq, Eq)]
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

#[record]
#[derive(Copy, PartialEq, Eq, Hash)]
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

    /// if this is a top level room permission
    ///
    /// these can only be set at the top level (ie. not as channel overwrites)
    pub fn is_top_level(&self) -> bool {
        todo!()
    }

    /// if this is a channel permission
    ///
    /// these can be overwritten for channels
    pub fn is_channel(&self) -> bool {
        todo!()
    }

    /// if this is a pack type room permission
    pub fn is_pack(&self) -> bool {
        matches!(
            self,
            Permission::Admin
                | Permission::IntegrationsManage
                | Permission::EmojiManage
                | Permission::InviteCreate
                | Permission::InviteManage
                | Permission::MemberBan
                | Permission::MemberKick
                | Permission::MemberNicknameManage
                | Permission::MemberNickname
                | Permission::MemberTimeout
                | Permission::RoleApply
                | Permission::RoleManage
                | Permission::RoomEdit
                | Permission::AuditLogView
                | Permission::AnalyticsView
        )
    }
}
