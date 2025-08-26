use common::v1::types::{
    util::Time, Bot, Embed, MediaId, MessageId, MessageType, MessageVerId, Permission, Puppet,
    Role, RoleId, RoleVerId, Room, RoomId, RoomMembership, RoomType, Session, SessionStatus,
    SessionToken, Thread, ThreadId, ThreadMembership, ThreadType, ThreadVerId, UserId,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

pub use common::v1::types::misc::{SessionIdReq, UserIdReq};

pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dm_uid_a: Option<Uuid>,
    pub dm_uid_b: Option<Uuid>,
    pub icon: Option<Uuid>,
    pub archived_at: Option<PrimitiveDateTime>,
    pub public: bool,
}

pub struct DbUserCreate {
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub bot: Option<Bot>,
    pub puppet: Option<Puppet>,
    pub registered_at: Option<Time>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum DbMembership {
    Join,
    Leave,
    Ban,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "room_type")]
pub enum DbRoomType {
    Default,
    Dm,
}

impl From<DbRoom> for Room {
    fn from(row: DbRoom) -> Self {
        #[allow(deprecated)]
        Room {
            id: row.id.into(),
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            icon: row.icon.map(|i| i.into()),
            room_type: if row.dm_uid_a.is_some() {
                RoomType::Dm {
                    participants: (row.dm_uid_a.unwrap().into(), row.dm_uid_b.unwrap().into()),
                }
            } else {
                RoomType::Default
            },
            archived_at: row.archived_at.map(|t| Time::from(t.assume_utc())),
            public: row.public,

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
}

#[derive(Deserialize, Clone)]
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
}

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "thread_type")]
pub enum DbThreadType {
    Chat,
    Forum,
    Voice,
    Dm,
}

impl From<DbThreadType> for ThreadType {
    fn from(value: DbThreadType) -> Self {
        match value {
            DbThreadType::Chat => ThreadType::Chat,
            DbThreadType::Forum => ThreadType::Forum,
            DbThreadType::Voice => ThreadType::Voice,
            DbThreadType::Dm => ThreadType::Dm,
        }
    }
}

impl From<DbThread> for Thread {
    fn from(row: DbThread) -> Self {
        dbg!(&row);
        Thread {
            id: row.id,
            room_id: row.room_id.map(Into::into),
            creator_id: row.creator_id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            nsfw: row.nsfw,

            member_count: row.member_count.try_into().expect("count is negative?"),
            // FIXME: calculate field
            online_count: 0,
            tags: Default::default(),
            permission_overwrites: serde_json::from_value(row.permission_overwrites).unwrap(),
            archived_at: None,
            deleted_at: None,
            locked: false,
            parent_id: None,
            position: None,

            ty: row.ty.into(),
            last_version_id: row.last_version_id.map(|i| i.into()),
            message_count: Some(row.message_count.try_into().expect("count is negative?")),
            root_message_count: None, // TODO
            bitrate: if row.ty == DbThreadType::Voice {
                Some(64000)
            } else {
                None
            },
            user_limit: if row.ty == DbThreadType::Voice {
                Some(100)
            } else {
                None
            },
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            notifications: None,
        }
    }
}

pub struct DbSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub token: SessionToken,
    pub status: DbSessionStatus,
    pub name: Option<String>,
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
                    expires_at: Time::now_utc(),
                },
            },
            name: row.name,
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

pub struct DbRole {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<DbPermission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
}

impl From<DbRole> for Role {
    fn from(row: DbRole) -> Self {
        Role {
            id: row.id,
            version_id: row.version_id,
            room_id: row.room_id,
            name: row.name,
            description: row.description,
            permissions: row.permissions.into_iter().map(Into::into).collect(),
            is_self_applicable: row.is_self_applicable,
            is_mentionable: row.is_mentionable,
            member_count: 0, // Placeholder, will be populated by the query
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

// TODO: move to types
impl DbMessageCreate {
    pub fn content(&self) -> Option<String> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.content.clone(),
            MessageType::DefaultTagged(msg) => msg.content.clone(),
            MessageType::ThreadUpdate(_patch) => Some("(thread update)".to_owned()),
            _ => None,
        }
    }

    pub fn metadata(&self) -> Option<serde_json::Value> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.metadata.clone(),
            MessageType::ThreadUpdate(patch) => Some(serde_json::to_value(patch).ok()?),
            _ => None,
        }
    }

    pub fn reply_id(&self) -> Option<MessageId> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.reply_id,
            MessageType::DefaultTagged(msg) => msg.reply_id,
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
    BotsAdd,
    BotsManage,
    EmojiAdd,
    EmojiManage,
    EmojiUseExternal,
    InviteCreate,
    InviteManage,
    MemberBan,
    MemberBanManage,
    MemberBridge,
    MemberKick,
    MemberManage,
    MessageCreate,
    MessageDelete,
    MessageEdit,
    MessageEmbeds,
    MessageMassMention,
    MessageAttachments,
    MessageMove,
    MessagePin,
    ReactionAdd,
    ReactionClear,
    ProfileAvatar,
    ProfileOverride,
    RoleApply,
    RoleManage,
    RoomManage,
    ServerAdmin,
    ServerMetrics,
    ServerOversee,
    ServerReports,
    TagApply,
    TagManage,
    ThreadArchive,
    ThreadCreateChat,
    ThreadCreateDocument,
    ThreadCreateEvent,
    ThreadCreateForumLinear,
    ThreadCreateForumTree,
    ThreadCreateTable,
    ThreadCreateVoice,
    ThreadCreatePublic,
    ThreadCreatePrivate,
    ThreadDelete,
    ThreadEdit,
    ThreadForward,
    ThreadLock,
    ThreadPin,
    ThreadPublish,
    UserDms,
    UserProfile,
    UserSessions,
    UserStatus,
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

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "thread_state")]
pub enum DbThreadState {
    Pinned,
    Active,
    Temporary,
    Archived,
    Deleted,
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
