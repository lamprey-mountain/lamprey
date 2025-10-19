use common::v1::types::Mentions;
use common::v1::types::{
    util::Time, Channel, ChannelId, ChannelType, ChannelVerId, Embed, MediaId, MessageId,
    MessageType, MessageVerId, Permission, Puppet, RoleId, Room, RoomId, RoomMembership, RoomType,
    Session, SessionStatus, SessionToken, SessionType, ThreadMembership, UserId,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub use common::v1::types::ids::*;
pub use common::v1::types::misc::{SessionIdReq, UserIdReq};

pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Uuid>,
    pub archived_at: Option<PrimitiveDateTime>,
    pub public: bool,
    pub ty: DbRoomType,
    pub welcome_channel_id: Option<Uuid>,
    pub member_count: i64,
    pub channel_count: i64,
    pub quarantined: bool,
}

pub struct DbRoomCreate {
    pub id: Option<RoomId>,
    pub ty: RoomType,
    pub welcome_channel_id: Option<ChannelId>,
}

pub struct DbUserCreate {
    pub id: Option<UserId>,
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub puppet: Option<Puppet>,
    pub registered_at: Option<Time>,
    pub system: bool,
}

#[derive(sqlx::Type, PartialEq)]
#[sqlx(type_name = "membership")]
pub enum DbMembership {
    Join,
    Leave,
    Ban, // unused
}

impl From<DbRoom> for Room {
    fn from(row: DbRoom) -> Self {
        #[allow(deprecated)]
        Room {
            id: row.id.into(),
            version_id: row.version_id,
            owner_id: row.owner_id.map(Into::into),
            name: row.name,
            description: row.description,
            icon: row.icon.map(|i| i.into()),
            room_type: row.ty.into(),
            archived_at: row.archived_at.map(|t| Time::from(t.assume_utc())),
            public: row.public,
            welcome_channel_id: row.welcome_channel_id.map(|i| i.into()),
            quarantined: row.quarantined,
            member_count: row.member_count as u64,
            online_count: Default::default(),
            channel_count: row.channel_count as u64,
            user_config: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DbChannel {
    pub id: ChannelId,
    pub room_id: Option<Uuid>,
    pub creator_id: UserId,
    pub owner_id: Option<Uuid>,
    pub version_id: ChannelVerId,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Uuid>,
    pub ty: DbChannelType,
    pub last_version_id: Option<Uuid>,
    pub message_count: i64,
    pub member_count: i64,
    pub permission_overwrites: serde_json::Value,
    pub nsfw: bool,
    pub locked: bool,
    pub archived_at: Option<PrimitiveDateTime>,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub parent_id: Option<Uuid>,
    pub position: Option<i32>,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbChannelPrivate {
    pub id: ChannelId,
    pub ty: DbChannelType,
    pub last_read_id: Option<Uuid>,
    pub is_unread: bool,
}

pub struct DbChannelCreate {
    pub room_id: Option<Uuid>,
    pub creator_id: UserId,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Uuid>,
    pub ty: DbChannelType,
    pub nsfw: bool,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,
    pub parent_id: Option<Uuid>,
}

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "channel_type")]
pub enum DbChannelType {
    Text,
    Forum,
    Voice,
    Dm,
    Gdm,
    Category,
    ThreadPublic,
    ThreadPrivate,
}

impl From<DbChannelType> for ChannelType {
    fn from(value: DbChannelType) -> Self {
        match value {
            DbChannelType::Text => ChannelType::Text,
            DbChannelType::Forum => ChannelType::Forum,
            DbChannelType::Voice => ChannelType::Voice,
            DbChannelType::Dm => ChannelType::Dm,
            DbChannelType::Gdm => ChannelType::Gdm,
            DbChannelType::Category => ChannelType::Category,
            DbChannelType::ThreadPublic => ChannelType::ThreadPublic,
            DbChannelType::ThreadPrivate => ChannelType::ThreadPrivate,
        }
    }
}

impl From<DbChannel> for Channel {
    fn from(row: DbChannel) -> Self {
        Channel {
            id: row.id,
            room_id: row.room_id.map(Into::into),
            creator_id: row.creator_id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            icon: row.icon.map(|i| i.into()),
            nsfw: row.nsfw,
            locked: row.locked,
            member_count: row.member_count.try_into().expect("count is negative?"),
            permission_overwrites: serde_json::from_value(row.permission_overwrites).unwrap(),
            archived_at: row.archived_at.map(|t| t.into()),
            deleted_at: row.deleted_at.map(|t| t.into()),
            ty: row.ty.into(),
            last_version_id: row.last_version_id.map(|i| i.into()),
            message_count: Some(row.message_count.try_into().expect("count is negative?")),
            parent_id: row.parent_id.map(|i| i.into()),
            position: row
                .position
                .map(|p| p.try_into().expect("position is negative or overflows?")),
            bitrate: row.bitrate.map(|i| i as u64),
            user_limit: row.user_limit.map(|i| i as u64),
            owner_id: row.owner_id.map(|i| i.into()),

            // these fields get filled in later
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            recipients: vec![],
            user_config: None,
            online_count: 0,

            // TODO: store or calculate the fields below
            tags: Default::default(),
            root_message_count: None,
        }
    }
}

pub struct DbSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub token: SessionToken,
    pub status: DbSessionStatus,
    pub name: Option<String>,
    pub expires_at: Option<PrimitiveDateTime>,
    pub ty: String,
    pub application_id: Option<Uuid>,
    pub last_seen_at: PrimitiveDateTime,
}

pub struct DbSessionCreate {
    pub token: SessionToken,
    pub name: Option<String>,
    pub expires_at: Option<Time>,
    pub ty: SessionType,
    pub application_id: Option<ApplicationId>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "session_status")]
pub enum DbSessionStatus {
    Unauthorized,
    Authorized,
    Sudo,
}

impl From<DbSession> for Session {
    fn from(row: DbSession) -> Self {
        Session {
            id: row.id.into(),
            status: match row.status {
                DbSessionStatus::Unauthorized => SessionStatus::Unauthorized,
                DbSessionStatus::Authorized => SessionStatus::Authorized {
                    user_id: row.user_id.expect("invalid data in db!").into(),
                },
                DbSessionStatus::Sudo => SessionStatus::Sudo {
                    user_id: row.user_id.expect("invalid data in db!").into(),
                    sudo_expires_at: Time::now_utc(),
                },
            },
            name: row.name,
            expires_at: row.expires_at.map(|t| t.into()),
            ty: SessionType::from_str(&row.ty).unwrap_or(SessionType::User),
            app_id: row.application_id.map(Into::into),
            last_seen_at: row.last_seen_at.into(),
        }
    }
}

impl From<SessionStatus> for DbSessionStatus {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Unauthorized => DbSessionStatus::Unauthorized,
            SessionStatus::Authorized { .. } => DbSessionStatus::Authorized,
            SessionStatus::Sudo { .. } => DbSessionStatus::Sudo,
        }
    }
}

pub struct DbRoleCreate {
    pub id: RoleId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
}

pub struct DbMessageCreate {
    pub channel_id: ChannelId,
    pub attachment_ids: Vec<MediaId>,
    pub author_id: UserId,
    pub embeds: Vec<Embed>,
    pub message_type: MessageType,
    pub edited_at: Option<time::PrimitiveDateTime>,
    pub created_at: Option<time::PrimitiveDateTime>,
    pub mentions: Mentions,
}

// TODO: move to types?
impl DbMessageCreate {
    pub fn content(&self) -> Option<String> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.content.clone(),
            _ => None,
        }
    }

    pub fn metadata(&self) -> Option<serde_json::Value> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.metadata.clone(),
            MessageType::ThreadRename(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::MemberAdd(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::MemberRemove(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::MemberJoin => None,
            MessageType::MessagePinned(pinned) => Some(serde_json::to_value(pinned).ok()?),
            _ => None,
        }
    }

    pub fn reply_id(&self) -> Option<MessageId> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.reply_id,
            _ => None,
        }
    }

    pub fn override_name(&self) -> Option<String> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.override_name.clone(),
            _ => None,
        }
    }
}

macro_rules! impl_perms {
    ($($e:ident,)*) => {
        #[derive(Debug, sqlx::Type, PartialEq, Eq)]
        #[sqlx(type_name = "permission")]
        pub enum DbPermission {
            $($e,)*
        }

        impl From<DbPermission> for Permission {
            fn from(value: DbPermission) -> Self {
                match value {
                    $(DbPermission::$e => Permission::$e,)*
                }
            }
        }

        impl From<Permission> for DbPermission {
            fn from(value: Permission) -> Self {
                match value {
                    $(Permission::$e => DbPermission::$e,)*
                }
            }
        }
    }
}

// surely there's a better way without copypasta
impl_perms!(
    Admin,
    IntegrationsManage,
    EmojiManage,
    EmojiUseExternal,
    InviteCreate,
    InviteManage,
    MemberBan,
    MemberBridge,
    MemberKick,
    MemberNicknameManage,
    MemberTimeout,
    MessageCreate,
    MessageDelete,
    MessageRemove,
    MessageEmbeds,
    MessageMassMention,
    MessageAttachments,
    MessageMove,
    MessagePin,
    ReactionAdd,
    ReactionPurge,
    MemberNickname,
    RoleApply,
    RoleManage,
    RoomManage,
    ServerMetrics,
    ServerOversee,
    ServerReports,
    TagApply,
    TagManage,
    ThreadCreatePublic,
    ThreadCreatePrivate,
    ThreadEdit,
    ThreadLock,
    ThreadManage,
    ViewChannel,
    ViewAuditLog,
    VoiceConnect,
    VoiceDeafen,
    VoiceDisconnect,
    VoiceMove,
    VoiceMute,
    VoicePriority,
    VoiceSpeak,
    VoiceVideo,
    ChannelManage,
    ChannelEdit,
    CalendarEventManage,
);

impl From<RoomMembership> for DbMembership {
    fn from(value: RoomMembership) -> Self {
        match value {
            RoomMembership::Join => DbMembership::Join,
            RoomMembership::Leave => DbMembership::Leave,
        }
    }
}

impl From<ThreadMembership> for DbMembership {
    fn from(value: ThreadMembership) -> Self {
        match value {
            ThreadMembership::Join => DbMembership::Join,
            ThreadMembership::Leave => DbMembership::Leave,
        }
    }
}

pub struct DbInvite {
    pub code: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub created_at: time::PrimitiveDateTime,
    pub expires_at: Option<time::PrimitiveDateTime>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct RoleDeleteQuery {
    #[serde(default)]
    pub force: bool,
}

#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "media_link_type")]
pub enum MediaLinkType {
    Message,
    MessageVersion,
    AvatarUser,
    BannerUser,
    // TODO: rename to IconChannel
    IconThread,
    AvatarRoom,
    Embed,
    CustomEmoji,
}

// TODO: surely there's a better way than manually managing media links/references
pub struct MediaLink {
    pub media_id: MediaId,
    pub target_id: Uuid,
    pub link_type: MediaLinkType,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UrlEmbedQueue {
    pub id: Uuid,
    pub message_ref: Option<serde_json::Value>,
    pub user_id: Uuid,
    pub url: String,
    pub created_at: PrimitiveDateTime,
    pub claimed_at: Option<PrimitiveDateTime>,
    pub finished_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbNotification {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub reason: String,
    pub added_at: PrimitiveDateTime,
    pub read_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRef {
    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub thread_id: ChannelId,
}

#[derive(sqlx::FromRow)]
pub struct DbEmailQueue {
    pub id: Uuid,
    pub to_addr: String,
    pub from_addr: String,
    pub subject: String,
    pub plain_text_body: String,
    pub html_body: Option<String>,
}

pub enum EmailPurpose {
    /// log in ("magic link")
    Authn,

    /// reset password
    Reset,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "room_type")]
pub enum DbRoomType {
    Default,
    Server,
}

impl Into<DbRoomType> for RoomType {
    fn into(self) -> DbRoomType {
        match self {
            RoomType::Default => DbRoomType::Default,
            RoomType::Server => DbRoomType::Server,
        }
    }
}

impl Into<RoomType> for DbRoomType {
    fn into(self) -> RoomType {
        match self {
            DbRoomType::Default => RoomType::Default,
            DbRoomType::Server => RoomType::Server,
        }
    }
}
