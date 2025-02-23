use serde::Deserialize;
use types::{
    MediaId, MessageId, MessageType, MessageVerId, Permission, Role, RoleId, RoleVerId, Room, RoomId, RoomMembership, Session, SessionId, SessionStatus, SessionToken, Thread, ThreadId, ThreadInfo, ThreadMembership, ThreadState, ThreadVerId, ThreadVisibility, UserId
};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

pub struct DbUserCreate {
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub is_bot: bool,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum DbMembership {
    Join,
    Leave,
    Ban,
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
    pub version_id: ThreadVerId,
    pub name: String,
    pub description: Option<String>,
    pub last_version_id: MessageVerId,
    pub last_read_id: Option<Uuid>,
    pub message_count: i64,
    pub is_unread: bool,
    pub state: DbThreadState,
}

pub struct ThreadCreate {
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
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
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            info: ThreadInfo::Chat {
                is_unread: row.is_unread,
                last_version_id: row.last_version_id,
                last_read_id: row.last_read_id.map(Into::into),
                message_count: row.message_count.try_into().expect("count is negative?"),
            },
            state: match row.state {
                DbThreadState::Pinned => todo!(),
                DbThreadState::Active => ThreadState::Active,
                DbThreadState::Temporary => ThreadState::Temporary,
                DbThreadState::Archived => ThreadState::Archived,
                DbThreadState::Deleted => ThreadState::Deleted,
            },
            visibility: ThreadVisibility::Room,
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

impl From<RoomMembership> for DbMembership {
    fn from(value: RoomMembership) -> Self {
        match value {
            RoomMembership::Join { .. } => DbMembership::Join,
            RoomMembership::Ban {} => DbMembership::Ban,
            RoomMembership::Leave {} => DbMembership::Leave,
        }
    }
}

impl From<ThreadMembership> for DbMembership {
    fn from(value: ThreadMembership) -> Self {
        match value {
            ThreadMembership::Join { .. } => DbMembership::Join,
            ThreadMembership::Ban {} => DbMembership::Ban,
            ThreadMembership::Leave {} => DbMembership::Leave,
        }
    }
}

// TODO: ToSchema
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
}

// TODO: surely there's a better way than manually managing media links/references
pub struct MediaLink {
    pub media_id: MediaId,
    pub target_id: Uuid,
    pub link_type: MediaLinkType,
}
