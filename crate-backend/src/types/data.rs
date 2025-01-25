use serde::Deserialize;
use tokio::io::BufWriter;
use types::{
    Media, MediaCreate, MediaId, Message, MessageId, MessageType, MessageVerId, Permission, Role,
    RoleId, RoleVerId, Room, RoomId, RoomMembership, Session, SessionId, SessionStatus,
    SessionToken, Thread, ThreadId, User, UserId, UserVerId,
};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct DbUser {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    // email: Option<String>,
    // avatar: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
    pub is_system: bool,
}

pub struct UserCreate {
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
    pub is_system: bool,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum DbRoomMembership {
    Join,
    Ban,
}

impl From<DbRoomMembership> for RoomMembership {
    fn from(value: DbRoomMembership) -> Self {
        match value {
            DbRoomMembership::Join => RoomMembership::Join,
            DbRoomMembership::Ban => RoomMembership::Ban,
        }
    }
}

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        User {
            id: row.id,
            version_id: row.version_id,
            parent_id: row.parent_id.map(UserId),
            name: row.name,
            description: row.description,
            status: row.status,
            is_bot: row.is_bot,
            is_alias: row.is_alias,
            is_system: row.is_system,
        }
    }
}

impl From<DbRoom> for Room {
    fn from(row: DbRoom) -> Self {
        Room {
            id: row.id.into(),
            version_id: row.version_id,
            name: row.name,
            description: row.description,
        }
    }
}

#[derive(Deserialize)]
pub struct DbThread {
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub is_closed: bool,
    pub is_locked: bool,
    pub is_pinned: bool,
    pub is_unread: bool,
    pub last_version_id: MessageId,
    pub last_read_id: Option<Uuid>,
    pub message_count: i64,
}

pub struct ThreadCreate {
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub is_closed: bool,
    pub is_locked: bool,
    pub is_pinned: bool,
}

// #[sqlx(type_name = "thread_type")]
// pub enum ThreadType {
// 	Default,
// }

impl From<DbThread> for Thread {
    fn from(row: DbThread) -> Self {
        Thread {
            id: row.id,
            room_id: row.room_id,
            creator_id: row.creator_id,
            name: row.name,
            description: row.description,
            is_closed: row.is_closed,
            is_locked: row.is_locked,
            is_pinned: row.is_pinned,
            is_unread: row.is_unread,
            last_version_id: row.last_version_id,
            last_read_id: row.last_read_id.map(Into::into),
            message_count: row.message_count.try_into().expect("count is negative?"),
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
    pub is_default: bool,
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
            is_default: row.is_default,
        }
    }
}

pub struct RoleCreate {
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

pub struct DbMessage {
    pub message_type: DbMessageType,
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub version_id: MessageVerId,
    pub ordering: i32,
    pub content: Option<String>,
    pub attachments: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>, // temp?
    pub author: serde_json::Value,
    pub is_pinned: bool,
}

pub struct MessageCreate {
    pub message_type: MessageType,
    pub thread_id: ThreadId,
    pub content: Option<String>,
    pub attachment_ids: Vec<MediaId>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    pub author_id: UserId,
    pub override_name: Option<String>, // temp?
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum DbMessageType {
    Default,
    ThreadUpdate,
}

impl From<DbMessageType> for MessageType {
    fn from(value: DbMessageType) -> Self {
        match value {
            DbMessageType::Default => MessageType::Default,
            DbMessageType::ThreadUpdate => MessageType::ThreadUpdate,
        }
    }
}

impl From<MessageType> for DbMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::Default => DbMessageType::Default,
            MessageType::ThreadUpdate => DbMessageType::ThreadUpdate,
        }
    }
}

impl From<DbMessage> for Message {
    fn from(row: DbMessage) -> Self {
        Message {
            id: row.id,
            message_type: row.message_type.into(),
            thread_id: row.thread_id,
            version_id: row.version_id,
            nonce: None,
            ordering: row.ordering,
            content: row.content,
            attachments: serde_json::from_value(row.attachments)
                .expect("invalid data in database!"),
            metadata: row.metadata,
            reply_id: row.reply_id.map(Into::into),
            override_name: row.override_name,
            author: serde_json::from_value(row.author).expect("invalid data in database!"),
            is_pinned: row.is_pinned,
        }
    }
}

use async_tempfile::TempFile;

pub struct MediaRow {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub source_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub mime: String,
    pub alt: Option<String>,
    pub size: i64,
    pub height: Option<i64>,
    pub width: Option<i64>,
    pub duration: Option<i64>,
}

pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
}

#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "media_link_type")]
pub enum MediaLinkType {
    Message,
    MessageVersion,
}

pub struct MediaLink {
    pub media_id: MediaId,
    pub target_id: Uuid,
    pub link_type: MediaLinkType,
}

impl From<MediaRow> for Media {
    fn from(row: MediaRow) -> Self {
        Media {
            id: row.id.into(),
            filename: row.filename,
            url: row.url,
            source_url: row.source_url,
            thumbnail_url: row.thumbnail_url,
            mime: row.mime,
            alt: row.alt,
            size: row.size.try_into().expect("database has negative size"),
            height: row
                .height
                .map(|i| i.try_into().expect("database has negative height")),
            width: row
                .width
                .map(|i| i.try_into().expect("database has negative width")),
            duration: row
                .duration
                .map(|i| i.try_into().expect("database has negative duration")),
        }
    }
}

// surely there's a better way
#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "permission")]
pub enum DbPermission {
    Admin,
    RoomManage,
    ThreadCreate,
    ThreadManage,
    ThreadDelete,
    MessageCreate,
    MessageFilesEmbeds,
    MessagePin,
    MessageDelete,
    MessageMassMention,
    MemberKick,
    MemberBan,
    MemberManage,
    InviteCreate,
    InviteManage,
    RoleManage,
    RoleApply,
    View,
    MessageEdit,
}

impl From<DbPermission> for Permission {
    fn from(value: DbPermission) -> Self {
        match value {
            DbPermission::Admin => Permission::Admin,
            DbPermission::RoomManage => Permission::RoomManage,
            DbPermission::ThreadCreate => Permission::ThreadCreate,
            DbPermission::ThreadManage => Permission::ThreadManage,
            DbPermission::ThreadDelete => Permission::ThreadDelete,
            DbPermission::MessageCreate => Permission::MessageCreate,
            DbPermission::MessageFilesEmbeds => Permission::MessageFilesEmbeds,
            DbPermission::MessagePin => Permission::MessagePin,
            DbPermission::MessageDelete => Permission::MessageDelete,
            DbPermission::MessageMassMention => Permission::MessageMassMention,
            DbPermission::MemberKick => Permission::MemberKick,
            DbPermission::MemberBan => Permission::MemberBan,
            DbPermission::MemberManage => Permission::MemberManage,
            DbPermission::InviteCreate => Permission::InviteCreate,
            DbPermission::InviteManage => Permission::InviteManage,
            DbPermission::RoleManage => Permission::RoleManage,
            DbPermission::RoleApply => Permission::RoleApply,
            DbPermission::View => Permission::View,
            DbPermission::MessageEdit => Permission::MessageEdit,
        }
    }
}

impl From<Permission> for DbPermission {
    fn from(value: Permission) -> Self {
        match value {
            Permission::Admin => DbPermission::Admin,
            Permission::RoomManage => DbPermission::RoomManage,
            Permission::ThreadCreate => DbPermission::ThreadCreate,
            Permission::ThreadManage => DbPermission::ThreadManage,
            Permission::ThreadDelete => DbPermission::ThreadDelete,
            Permission::MessageCreate => DbPermission::MessageCreate,
            Permission::MessageFilesEmbeds => DbPermission::MessageFilesEmbeds,
            Permission::MessagePin => DbPermission::MessagePin,
            Permission::MessageDelete => DbPermission::MessageDelete,
            Permission::MessageMassMention => DbPermission::MessageMassMention,
            Permission::MemberKick => DbPermission::MemberKick,
            Permission::MemberBan => DbPermission::MemberBan,
            Permission::MemberManage => DbPermission::MemberManage,
            Permission::InviteCreate => DbPermission::InviteCreate,
            Permission::InviteManage => DbPermission::InviteManage,
            Permission::RoleManage => DbPermission::RoleManage,
            Permission::RoleApply => DbPermission::RoleApply,
            Permission::View => DbPermission::View,
            Permission::MessageEdit => DbPermission::MessageEdit,
        }
    }
}

impl From<RoomMembership> for DbRoomMembership {
    fn from(value: RoomMembership) -> Self {
        match value {
            RoomMembership::Join => DbRoomMembership::Join,
            RoomMembership::Ban => DbRoomMembership::Ban,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UserIdReq {
    #[serde(deserialize_with = "const_self")]
    UserSelf,
    UserId(UserId),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SessionIdReq {
    #[serde(deserialize_with = "const_self")]
    SessionSelf,
    // #[serde(deserialize_with = "const_all")]
    // SessionAll,
    SessionId(SessionId),
}

fn const_self<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "@self")]
        Variant,
    }

    Helper::deserialize(deserializer).map(|_| ())
}

// fn const_all<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     #[derive(Deserialize)]
//     enum Helper {
//         #[serde(rename = "@all")]
//         Variant,
//     }

//     Helper::deserialize(deserializer).map(|_| ())
// }

pub struct DbInvite {
    pub code: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub creator_id: Uuid,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub created_at: time::PrimitiveDateTime,
    pub expires_at: Option<time::PrimitiveDateTime>,
}
