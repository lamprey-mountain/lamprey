use common::v1::types::{
    util::Time, Bot, Embed, MediaId, MessageId, MessageType, MessageVerId, Permission, Puppet,
    RoleId, Room, RoomId, RoomMembership, RoomType, Session, SessionStatus, SessionToken,
    SessionType, Thread, ThreadId, ThreadMembership, ThreadType, ThreadVerId, UserId,
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
    pub welcome_thread_id: Option<Uuid>,
}

pub struct DbRoomCreate {
    pub id: Option<RoomId>,
    pub ty: RoomType,
}

pub struct DbUserCreate {
    pub id: Option<UserId>,
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub bot: Option<Bot>,
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
            welcome_thread_id: row.welcome_thread_id.map(|i| i.into()),

            // FIXME: add to db or calculate
            member_count: Default::default(),
            online_count: Default::default(),
            thread_count: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DbThread {
    pub id: ThreadId,
    pub room_id: Option<Uuid>,
    pub creator_id: UserId,
    pub version_id: ThreadVerId,
    pub name: String,
    pub description: Option<String>,
    pub ty: DbThreadType,
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
pub struct DbThreadPrivate {
    pub id: ThreadId,
    pub ty: DbThreadType,
    pub last_read_id: Option<Uuid>,
    pub is_unread: bool,
}

pub struct DbThreadCreate {
    pub room_id: Option<Uuid>,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub ty: DbThreadType,
    pub nsfw: bool,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,
}

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "thread_type")]
pub enum DbThreadType {
    Chat,
    Forum,
    Voice,
    Dm,
    Gdm,
    Category,
}

impl From<DbThreadType> for ThreadType {
    fn from(value: DbThreadType) -> Self {
        match value {
            DbThreadType::Chat => ThreadType::Chat,
            DbThreadType::Forum => ThreadType::Forum,
            DbThreadType::Voice => ThreadType::Voice,
            DbThreadType::Dm => ThreadType::Dm,
            DbThreadType::Gdm => ThreadType::Gdm,
            DbThreadType::Category => ThreadType::Category,
        }
    }
}

impl From<DbThread> for Thread {
    fn from(row: DbThread) -> Self {
        Thread {
            id: row.id,
            room_id: row.room_id.map(Into::into),
            creator_id: row.creator_id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
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

            // these fields get filled in later
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            notifications: None,
            recipient: None,
            recipients: vec![],

            // TODO: store or calculate the fields below
            tags: Default::default(),
            online_count: 0,
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
    pub thread_id: ThreadId,
    pub attachment_ids: Vec<MediaId>,
    pub author_id: UserId,
    pub embeds: Vec<Embed>,
    pub message_type: MessageType,
    pub edited_at: Option<time::PrimitiveDateTime>,
    pub created_at: Option<time::PrimitiveDateTime>,
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
    ThreadArchive,
    ThreadCreateChat,
    ThreadCreateForum,
    ThreadCreateVoice,
    ThreadCreatePublic,
    ThreadCreatePrivate,
    ThreadRemove,
    ThreadEdit,
    ThreadForward,
    ThreadLock,
    ThreadManage,
    ThreadPublish,
    View,
    ViewAuditLog,
    VoiceConnect,
    VoiceDeafen,
    VoiceDisconnect,
    VoiceMove,
    VoiceMute,
    VoicePriority,
    VoiceSpeak,
    VoiceVideo,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRef {
    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub thread_id: ThreadId,
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
