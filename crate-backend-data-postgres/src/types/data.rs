// TODO: move into data mod

use common::v1::types::automod::{AutomodAction, AutomodTarget, AutomodTrigger};
use common::v1::types::calendar::{CalendarEvent, CalendarOverwrite};
use common::v1::types::document::DocumentBranchState;
use common::v1::types::User;
use common::v1::types::{
    util::Time, Channel, ChannelType, Embed, Permission, Puppet, Room, RoomType, Session,
    SessionStatus, SessionToken, SessionType,
};
use common::v1::types::{AuditLogEntryStatus, Mentions, RoomSecurity};
use common::v2::types::message::MessageType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub use common::v1::types::ids::*;
pub use common::v1::types::misc::{SessionIdReq, UserIdReq};

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum DbMessageType {
    DefaultMarkdown,
    DefaultTagged, // removed
    ThreadUpdate,  // removed
    MemberAdd,
    MemberRemove,
    MemberJoin,
    MessagePinned,
    ThreadCreated,
    ChannelRename,
    ChannelIcon,
    ChannelPingback,
    ChannelMoved,
    AutomodExecution,
    Call,
}

impl From<MessageType> for DbMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::DefaultMarkdown(_) => DbMessageType::DefaultMarkdown,
            MessageType::ChannelRename(_) => DbMessageType::ChannelRename,
            MessageType::MemberAdd(_) => DbMessageType::MemberAdd,
            MessageType::MemberRemove(_) => DbMessageType::MemberRemove,
            MessageType::MemberJoin => DbMessageType::MemberJoin,
            MessageType::Call(_) => DbMessageType::Call,
            MessageType::MessagePinned(_) => DbMessageType::MessagePinned,
            MessageType::ThreadCreated(_) => DbMessageType::ThreadCreated,
            MessageType::ChannelIcon(_) => DbMessageType::ChannelIcon,
            MessageType::ChannelPingback(_) => DbMessageType::ChannelPingback,
            MessageType::ChannelMoved(_) => DbMessageType::ChannelMoved,
            MessageType::AutomodExecution(_) => DbMessageType::AutomodExecution,
        }
    }
}

// deserialize from jsonb
#[derive(Debug, Serialize, Deserialize)]
pub struct AutomodRuleData {
    pub trigger: AutomodTrigger,
    pub actions: Vec<AutomodAction>,
}

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "automod_target")]
pub enum DbAutomodTarget {
    Content,
    Member,
}

impl From<DbAutomodTarget> for AutomodTarget {
    fn from(value: DbAutomodTarget) -> Self {
        match value {
            DbAutomodTarget::Content => AutomodTarget::Content,
            DbAutomodTarget::Member => AutomodTarget::Member,
        }
    }
}

impl From<AutomodTarget> for DbAutomodTarget {
    fn from(value: AutomodTarget) -> Self {
        match value {
            AutomodTarget::Content => DbAutomodTarget::Content,
            AutomodTarget::Member => DbAutomodTarget::Member,
        }
    }
}

pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Uuid>,
    pub banner: Option<Uuid>,
    pub archived_at: Option<PrimitiveDateTime>,
    pub public: bool,
    pub ty: DbRoomType,
    pub welcome_channel_id: Option<Uuid>,
    pub member_count: i64,
    pub channel_count: i64,
    pub emoji_count: i64,
    pub quarantined: bool,
    pub security_require_mfa: bool,
    pub security_require_sudo: bool,
    pub afk_channel_id: Option<Uuid>,
    pub afk_channel_timeout: i64,
    pub deleted_at: Option<PrimitiveDateTime>,
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
            banner: row.banner.map(|i| i.into()),
            room_type: row.ty.into(),
            archived_at: row.archived_at.map(|t| Time::from(t.assume_utc())),
            public: row.public,
            deleted_at: row.deleted_at.map(|t| Time::from(t.assume_utc())),
            welcome_channel_id: row.welcome_channel_id.map(|i| i.into()),
            quarantined: row.quarantined,
            member_count: row.member_count as u64,
            online_count: 0,
            channel_count: row.channel_count as u64,
            emoji_count: row.emoji_count as u64,
            preferences: None,
            security: RoomSecurity {
                require_mfa: row.security_require_mfa,
                require_sudo: row.security_require_sudo,
            },
            afk_channel_id: row.afk_channel_id.map(|i| i.into()),
            afk_channel_timeout: row.afk_channel_timeout as u64,
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
    pub url: Option<String>,
    pub icon: Option<Uuid>,
    pub ty: DbChannelType,
    pub last_version_id: Option<Uuid>,
    pub last_message_id: Option<Uuid>,
    pub message_count: i64,
    pub member_count: i64,
    pub permission_overwrites: serde_json::Value,
    pub nsfw: bool,
    pub locked: bool,
    pub locked_until: Option<PrimitiveDateTime>,
    pub locked_roles: Vec<Uuid>,
    pub archived_at: Option<PrimitiveDateTime>,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub parent_id: Option<Uuid>,
    pub position: Option<i32>,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,
    pub tags: Option<serde_json::Value>,
    pub tags_available: Option<serde_json::Value>,
    pub tag_count: i64,
    pub invitable: bool,
    pub auto_archive_duration: Option<i64>,
    pub default_auto_archive_duration: Option<i64>,
    pub slowmode_thread: Option<i32>,
    pub slowmode_message: Option<i32>,
    pub default_slowmode_message: Option<i32>,
    pub last_activity_at: Option<PrimitiveDateTime>,
    pub document: Option<serde_json::Value>,
    pub wiki: Option<serde_json::Value>,
    pub calendar: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbChannelPrivate {
    pub id: ChannelId,
    pub ty: DbChannelType,
    pub last_read_id: Option<Uuid>,
    pub is_unread: bool,
    pub mention_count: i64,
}

pub struct DbChannelCreate {
    pub room_id: Option<Uuid>,
    pub creator_id: UserId,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub icon: Option<Uuid>,
    pub ty: DbChannelType,
    pub nsfw: bool,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,
    pub parent_id: Option<Uuid>,
    pub invitable: bool,
    pub auto_archive_duration: Option<i64>,
    pub default_auto_archive_duration: Option<i64>,
    pub slowmode_thread: Option<i64>,
    pub slowmode_message: Option<i64>,
    pub default_slowmode_message: Option<i64>,
    pub locked: bool,
    pub tags: Option<Vec<TagId>>,
}

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "channel_type")]
pub enum DbChannelType {
    Text,
    Announcement,
    Forum,
    Forum2,
    Voice,
    Broadcast,
    Dm,
    Gdm,
    Category,
    ThreadPublic,
    ThreadPrivate,
    ThreadForum2,
    Calendar,
    Info,
    Ticket,
    Wiki,
    Document,
    DocumentComment,
}

impl From<DbChannelType> for ChannelType {
    fn from(value: DbChannelType) -> Self {
        match value {
            DbChannelType::Text => ChannelType::Text,
            DbChannelType::Announcement => ChannelType::Announcement,
            DbChannelType::Forum => ChannelType::Forum,
            DbChannelType::Forum2 => ChannelType::Forum2,
            DbChannelType::Voice => ChannelType::Voice,
            DbChannelType::Broadcast => ChannelType::Broadcast,
            DbChannelType::Dm => ChannelType::Dm,
            DbChannelType::Gdm => ChannelType::Gdm,
            DbChannelType::Category => ChannelType::Category,
            DbChannelType::ThreadPublic => ChannelType::ThreadPublic,
            DbChannelType::ThreadPrivate => ChannelType::ThreadPrivate,
            DbChannelType::ThreadForum2 => ChannelType::ThreadForum2,
            DbChannelType::Calendar => ChannelType::Calendar,
            DbChannelType::Info => ChannelType::Info,
            DbChannelType::Ticket => ChannelType::Ticket,
            DbChannelType::Wiki => ChannelType::Wiki,
            DbChannelType::Document => ChannelType::Document,
            DbChannelType::DocumentComment => ChannelType::DocumentComment,
        }
    }
}

impl From<ChannelType> for DbChannelType {
    fn from(value: ChannelType) -> Self {
        match value {
            ChannelType::Text => DbChannelType::Text,
            ChannelType::Announcement => DbChannelType::Announcement,
            ChannelType::Forum => DbChannelType::Forum,
            ChannelType::Forum2 => DbChannelType::Forum2,
            ChannelType::Voice => DbChannelType::Voice,
            ChannelType::Broadcast => DbChannelType::Broadcast,
            ChannelType::Dm => DbChannelType::Dm,
            ChannelType::Gdm => DbChannelType::Gdm,
            ChannelType::Category => DbChannelType::Category,
            ChannelType::ThreadPublic => DbChannelType::ThreadPublic,
            ChannelType::ThreadPrivate => DbChannelType::ThreadPrivate,
            ChannelType::ThreadForum2 => DbChannelType::ThreadForum2,
            ChannelType::Calendar => DbChannelType::Calendar,
            ChannelType::Info => DbChannelType::Info,
            ChannelType::Ticket => DbChannelType::Ticket,
            ChannelType::Wiki => DbChannelType::Wiki,
            ChannelType::Document => DbChannelType::Document,
            ChannelType::DocumentComment => DbChannelType::DocumentComment,
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
            locked: if row.locked {
                Some(common::v1::types::channel::Locked {
                    until: row.locked_until.map(Into::into),
                    allow_roles: row.locked_roles.into_iter().map(Into::into).collect(),
                })
            } else {
                None
            },
            member_count: row.member_count.try_into().expect("count is negative?"),
            permission_overwrites: serde_json::from_value(row.permission_overwrites).unwrap(),
            archived_at: row.archived_at.map(|t| t.into()),
            deleted_at: row.deleted_at.map(|t| t.into()),
            ty: row.ty.into(),
            last_version_id: row.last_version_id.map(|i| i.into()),
            last_message_id: row.last_message_id.map(|i| i.into()),
            message_count: Some(row.message_count.try_into().expect("count is negative?")),
            parent_id: row.parent_id.map(|i| i.into()),
            position: row
                .position
                .map(|p| p.try_into().expect("position is negative or overflows?")),
            bitrate: row.bitrate.map(|i| i as u64),
            user_limit: row.user_limit.map(|i| i as u64),
            owner_id: row.owner_id.map(|i| i.into()),
            invitable: row.invitable,
            auto_archive_duration: row.auto_archive_duration.map(|i| i as u64),
            default_auto_archive_duration: row.default_auto_archive_duration.map(|i| i as u64),
            tags: row
                .tags
                .map(|v| serde_json::from_value(v).unwrap_or_default()),
            tags_available: row
                .tags_available
                .map(|v| serde_json::from_value(v).unwrap_or_default()),
            tag_count: row.tag_count.try_into().expect("tag_count is negative?"),
            root_message_count: None,
            thread_member: None,
            slowmode_thread: row.slowmode_thread.map(|v| v as u64),
            slowmode_message: row.slowmode_message.map(|v| v as u64),
            default_slowmode_message: row.default_slowmode_message.map(|v| v as u64),
            url: row.url,
            document: row.document.and_then(|v| serde_json::from_value(v).ok()),
            wiki: row.wiki.and_then(|v| serde_json::from_value(v).ok()),
            calendar: row.calendar.and_then(|v| serde_json::from_value(v).ok()),

            // these fields get filled in later
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            recipients: vec![],
            preferences: None,
            online_count: 0,
            slowmode_thread_expire_at: None,
            slowmode_message_expire_at: None,
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
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,
    pub authorized_at: Option<PrimitiveDateTime>,
    pub deauthorized_at: Option<PrimitiveDateTime>,
}

pub struct DbSessionCreate {
    pub token: SessionToken,
    pub name: Option<String>,
    pub expires_at: Option<Time>,
    pub ty: SessionType,
    pub application_id: Option<ApplicationId>,
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "session_status")]
pub enum DbSessionStatus {
    Unauthorized,
    Bound,
    Authorized,
    Sudo,
}

impl From<DbSession> for Session {
    fn from(row: DbSession) -> Self {
        Session {
            id: row.id.into(),
            status: match row.status {
                DbSessionStatus::Unauthorized => SessionStatus::Unauthorized,
                DbSessionStatus::Bound => SessionStatus::Bound {
                    user_id: row.user_id.expect("invalid data in db!").into(),
                },
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
            ip_addr: row.ip_addr,
            user_agent: row.user_agent,
            authorized_at: row.authorized_at.map(|t| t.into()),
            deauthorized_at: row.deauthorized_at.map(|t| t.into()),
        }
    }
}

impl From<SessionStatus> for DbSessionStatus {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Unauthorized => DbSessionStatus::Unauthorized,
            SessionStatus::Bound { .. } => DbSessionStatus::Bound,
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
    pub allow: Vec<Permission>,
    pub deny: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub hoist: bool,
    pub sticky: bool,
}

/// for message_create
pub struct DbMessageCreate {
    pub id: Option<MessageId>,
    pub channel_id: ChannelId,
    pub attachment_ids: Vec<MediaId>,
    pub author_id: UserId,
    pub embeds: Vec<Embed>,
    pub message_type: MessageType,
    pub created_at: Option<time::PrimitiveDateTime>,
    pub removed_at: Option<time::PrimitiveDateTime>,
    pub mentions: Mentions,
}

/// for message_update, message_update_in_place
pub struct DbMessageUpdate {
    pub attachment_ids: Vec<MediaId>,
    pub author_id: UserId,
    pub embeds: Vec<Embed>,
    pub message_type: MessageType,
    pub created_at: Option<time::PrimitiveDateTime>,
    pub mentions: Mentions,
}

/// trait for extracting common message data
pub trait DbMessageExtract {
    fn content(&self) -> Option<String>;
    fn metadata(&self) -> Option<serde_json::Value>;
    fn reply_id(&self) -> Option<MessageId>;
    fn override_name(&self) -> Option<String>;
}

impl DbMessageExtract for MessageType {
    fn content(&self) -> Option<String> {
        match self {
            MessageType::DefaultMarkdown(msg) => msg.content.clone(),
            _ => None,
        }
    }

    fn metadata(&self) -> Option<serde_json::Value> {
        match self {
            MessageType::DefaultMarkdown(msg) => msg
                .metadata
                .as_ref()
                .and_then(|m| serde_json::to_value(m).ok()),
            MessageType::MemberAdd(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::MemberRemove(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::MemberJoin => None,
            MessageType::MessagePinned(pinned) => Some(serde_json::to_value(pinned).ok()?),
            MessageType::ChannelMoved(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::ChannelPingback(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::ChannelRename(patch) => Some(serde_json::to_value(patch).ok()?),
            MessageType::ChannelIcon(icon) => Some(serde_json::to_value(icon).ok()?),
            MessageType::ThreadCreated(created) => Some(serde_json::to_value(created).ok()?),
            MessageType::AutomodExecution(exec) => Some(serde_json::to_value(exec).ok()?),
            _ => None,
        }
    }

    fn reply_id(&self) -> Option<MessageId> {
        match self {
            MessageType::DefaultMarkdown(msg) => msg.reply_id,
            _ => None,
        }
    }

    fn override_name(&self) -> Option<String> {
        // v2 messages don't have override_name, return None
        None
    }
}

macro_rules! impl_perms {
    ($($e:ident,)*) => {
        #[derive(Debug, Clone, Copy, sqlx::Type, PartialEq, Eq)]
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
    ApplicationCreate,
    ApplicationManage,
    IntegrationsManage,
    IntegrationsBridge,
    EmojiManage,
    EmojiUseExternal,
    InviteCreate,
    InviteManage,
    MemberBan,
    MemberKick,
    MemberNickname,
    MemberNicknameManage,
    MemberTimeout,
    MessageAttachments,
    MessageCreate,
    MessageCreateThread,
    MessageDelete,
    MessageEmbeds,
    MessageMassMention,
    MessageMove,
    MessagePin,
    MessageRemove,
    ReactionAdd,
    ReactionManage,
    RoleApply,
    RoleManage,
    RoomEdit,
    RoomManage,
    ServerMaintenance,
    ServerMetrics,
    ServerOversee,
    ChannelSlowmodeBypass,
    ChannelEdit,
    ChannelManage,
    ThreadCreatePrivate,
    ThreadCreatePublic,
    ThreadManage,
    ThreadEdit,
    ChannelView,
    AuditLogView,
    AnalyticsView,
    VoiceDeafen,
    VoiceMove,
    VoiceMute,
    VoicePriority,
    VoiceSpeak,
    VoiceVideo,
    VoiceVad,
    VoiceRequest,
    VoiceBroadcast,
    CalendarEventCreate,
    CalendarEventRsvp,
    CalendarEventManage,
    DocumentCreate,
    DocumentEdit,
    DocumentComment,
    RoomCreate,
    UserManage,
    UserManageSelf,
    UserProfileSelf,
    DmCreate,
    FriendCreate,
    RoomJoin,
    CallUpdate,
    RoomJoinForce,
);

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

/// what object this media is linked to
///
/// normally one piece of media is linked to exactly one object, but a slightly
/// awkward thing to note is that media linked to `Message`s also have links to each
/// `MessageVersion` they're referenced in.
#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "media_link_type")]
pub enum MediaLinkType {
    Message,
    MessageVersion,
    UserAvatar,
    UserBanner,
    ChannelIcon,
    RoomIcon,
    RoomBanner,
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

#[derive(sqlx::Type, Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "notification_type")]
pub enum DbNotificationType {
    Message,
    Reaction,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbNotification {
    pub id: Uuid,
    pub room_id: Option<Uuid>,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub ty: DbNotificationType,
    pub added_at: PrimitiveDateTime,
    pub read_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// this is whats actually stored in the db
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MentionsIds {
    pub users: Vec<UserId>,
    pub roles: Vec<RoleId>,

    #[serde(default)]
    pub channels: Vec<ChannelId>,

    #[serde(default)]
    pub emojis: Vec<EmojiId>,

    #[serde(default)]
    pub everyone: bool,
}

impl Into<MentionsIds> for Mentions {
    fn into(self) -> MentionsIds {
        MentionsIds {
            users: self.users.into_iter().map(|mention| mention.id).collect(),
            roles: self.roles.into_iter().map(|mention| mention.id).collect(),
            channels: self
                .channels
                .into_iter()
                .map(|mention| mention.id)
                .collect(),
            emojis: self.emojis.into_iter().map(|mention| mention.id).collect(),
            everyone: self.everyone,
        }
    }
}

pub struct DehydratedDocument {
    pub last_snapshot: Vec<u8>,
    pub snapshot_seq: u32,
    pub changes: Vec<Vec<u8>>,
}

pub struct DocumentUpdateSummary {
    pub user_id: Uuid,
    pub created_at: Time,
    pub stat_added: u32,
    pub stat_removed: u32,
    pub seq: u32,
    pub document_id: ChannelId,
}

#[derive(sqlx::Type, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[sqlx(type_name = "audit_log_entry_status")]
pub enum DbAuditLogEntryStatus {
    Success,
    Unauthorized,
    Failed,
}

impl From<DbAuditLogEntryStatus> for AuditLogEntryStatus {
    fn from(value: DbAuditLogEntryStatus) -> Self {
        match value {
            DbAuditLogEntryStatus::Success => AuditLogEntryStatus::Success,
            DbAuditLogEntryStatus::Unauthorized => AuditLogEntryStatus::Unauthorized,
            DbAuditLogEntryStatus::Failed => AuditLogEntryStatus::Failed,
        }
    }
}

impl From<AuditLogEntryStatus> for DbAuditLogEntryStatus {
    fn from(value: AuditLogEntryStatus) -> Self {
        match value {
            AuditLogEntryStatus::Success => DbAuditLogEntryStatus::Success,
            AuditLogEntryStatus::Unauthorized => DbAuditLogEntryStatus::Unauthorized,
            AuditLogEntryStatus::Failed => DbAuditLogEntryStatus::Failed,
        }
    }
}

#[derive(Debug)]
pub struct DbChannelDocument {
    pub channel_id: Uuid,
    pub draft: bool,
    pub archived_at: Option<PrimitiveDateTime>,
    pub archived_reason: Option<String>,
    pub template: bool,
    pub slug: Option<String>,
    pub published_at: Option<PrimitiveDateTime>,
    pub published_revision: Option<String>,
    pub published_unlisted: Option<bool>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbRoomTemplate {
    pub code: String,
    pub name: String,
    pub description: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub creator_id: Uuid,
    pub source_room_id: Option<Uuid>,
    pub snapshot: serde_json::Value,
    pub dirty: bool,
}

#[derive(Debug)]
pub struct DbChannelWiki {
    pub channel_id: Uuid,
    pub allow_indexing: bool,
    pub page_index: Option<Uuid>,
    pub page_notfound: Option<Uuid>,
}

#[derive(Debug)]
pub struct DbChannelCalendar {
    pub channel_id: Uuid,
    pub color: Option<String>,
    pub default_timezone: String,
}

pub struct PushData {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub endpoint: String,
    pub key_p256dh: String,
    pub key_auth: String,
}

// Types moved from postgres implementation files

#[derive(sqlx::FromRow)]
pub struct DbUser {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<Uuid>,
    pub banner: Option<Uuid>,
    pub bot: bool,
    pub system: bool,
    pub suspended: Option<Value>,
    pub registered_at: Option<time::PrimitiveDateTime>,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub webhook_channel_id: Option<Uuid>,
    pub webhook_creator_id: Option<Uuid>,
    pub webhook_room_id: Option<Uuid>,
    pub puppet_application_id: Option<Uuid>,
    pub puppet_external_id: Option<String>,
    pub puppet_external_url: Option<String>,
    pub puppet_alias_id: Option<Uuid>,
}

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        let webhook = if let (Some(channel_id), Some(creator_id)) =
            (row.webhook_channel_id, row.webhook_creator_id)
        {
            Some(common::v1::types::UserWebhook {
                room_id: row.webhook_room_id.map(Into::into),
                channel_id: channel_id.into(),
                creator_id: creator_id.into(),
            })
        } else {
            None
        };

        Self {
            id: row.id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            avatar: row.avatar.map(Into::into),
            banner: row.banner.map(Into::into),
            bot: row.bot,
            system: row.system,
            puppet: None,
            webhook,
            suspended: None,
            presence: common::v1::types::presence::Presence::offline(),
            registered_at: row.registered_at.map(|t| t.into()),
            deleted_at: row.deleted_at.map(|t| t.into()),
            emails: None,
            preferences: None,
            has_mfa: None,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct DbRoomMember {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub membership: DbMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    pub joined_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
    pub origin: Option<serde_json::Value>,
    pub mute: bool,
    pub deaf: bool,
    pub timeout_until: Option<time::PrimitiveDateTime>,
    pub quarantined: bool,
}

#[derive(sqlx::FromRow)]
pub struct DbRoomBan {
    pub user_id: UserId,
    pub reason: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub expires_at: Option<time::PrimitiveDateTime>,
}

impl From<DbRoomBan> for common::v1::types::RoomBan {
    fn from(row: DbRoomBan) -> Self {
        Self {
            user_id: row.user_id,
            reason: row.reason,
            created_at: row.created_at.assume_utc().into(),
            expires_at: row.expires_at.map(|t| t.assume_utc().into()),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct DbThreadMember {
    pub thread_id: Uuid,
    pub user_id: UserId,
    pub last_read_at: Option<time::PrimitiveDateTime>,
    pub acknowledged_at: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "branch_state")]
pub enum DbBranchState {
    Active,
    Closed,
    Merged,
}

impl From<DocumentBranchState> for DbBranchState {
    fn from(value: DocumentBranchState) -> Self {
        match value {
            DocumentBranchState::Active => DbBranchState::Active,
            DocumentBranchState::Closed => DbBranchState::Closed,
            DocumentBranchState::Merged => DbBranchState::Merged,
        }
    }
}

impl From<DbBranchState> for DocumentBranchState {
    fn from(value: DbBranchState) -> Self {
        match value {
            DbBranchState::Active => DocumentBranchState::Active,
            DbBranchState::Closed => DocumentBranchState::Closed,
            DbBranchState::Merged => DocumentBranchState::Merged,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct DbCalendarEvent {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub timezone: Option<String>,
    pub recurrence: Option<serde_json::Value>,
    pub start_at: PrimitiveDateTime,
    pub end_at: Option<PrimitiveDateTime>,
}

impl From<DbCalendarEvent> for CalendarEvent {
    fn from(val: DbCalendarEvent) -> Self {
        Self {
            id: val.id.into(),
            channel_id: val.channel_id.into(),
            creator_id: val.creator_id.map(|i| i.into()),
            title: val.title,
            description: val.description,
            location: val.location,
            url: val.url.and_then(|u| u.parse().ok()),
            timezone: val.timezone.map(common::v1::types::calendar::Timezone),
            recurrence: val.recurrence.and_then(|v| serde_json::from_value(v).ok()),
            starts_at: val.start_at.into(),
            ends_at: val.end_at.map(|e| e.into()),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct DbCalendarOverwrite {
    pub event_id: Uuid,
    pub seq: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub start_at: Option<PrimitiveDateTime>,
    pub end_at: Option<PrimitiveDateTime>,
    pub cancelled: bool,
}

impl From<DbCalendarOverwrite> for CalendarOverwrite {
    fn from(val: DbCalendarOverwrite) -> Self {
        Self {
            event_id: val.event_id.into(),
            seq: val.seq as u64,
            title: val.title,
            extra_description: val.description,
            location: val.location.map(Some),
            url: val.url.and_then(|u| u.parse().ok()).map(Some),
            starts_at: val.start_at.map(Into::into),
            ends_at: val.end_at.map(|e| Some(e.into())),
            cancelled: val.cancelled,
        }
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "media_type")]
pub enum DbMediaType {
    Image,
    Video,
    Audio,
    File,
}

#[derive(sqlx::FromRow)]
pub struct DbMedia {
    pub id: MediaId,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub duration: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub media_type: DbMediaType,
}

#[derive(sqlx::FromRow)]
pub struct DbMediaWithIdNew {
    pub id: MediaId,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub duration: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub media_type: DbMediaType,
    pub data: Vec<u8>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "media_data")]
pub struct DbMediaDataNew {
    pub data: Vec<u8>,
    pub s3_url: String,
}

#[derive(sqlx::FromRow)]
pub struct DbMediaRaw {
    pub id: MediaId,
    pub data: Vec<u8>,
}

pub type EditContextId = (ChannelId, DocumentBranchId);

// Additional types needed by crate-backend

#[derive(sqlx::FromRow)]
pub struct DbMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub room_id: Option<Uuid>,
    pub author_id: UserId,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub removed_at: Option<time::PrimitiveDateTime>,
    pub pinned: Option<serde_json::Value>,
    pub message_type: DbMessageType,
    pub version_id: MessageVerId,
    pub version_author_id: UserId,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>,
    pub embeds: Option<serde_json::Value>,
    pub version_created_at: time::PrimitiveDateTime,
    pub version_deleted_at: Option<time::PrimitiveDateTime>,
    pub attachments: serde_json::Value,
}

#[derive(sqlx::FromRow)]
pub struct DbMessageVersion {
    pub id: MessageId,
    pub seq: i64,
    pub channel_id: ChannelId,
    pub version_id: MessageVerId,
    pub author_id: UserId,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub message_type: DbMessageType,
    pub attachments: serde_json::Value,
}

#[derive(sqlx::FromRow)]
pub struct DbRoomMemberWithUser {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub membership: DbMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    pub joined_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
    pub origin: Option<serde_json::Value>,
    pub mute: bool,
    pub deaf: bool,
    pub timeout_until: Option<time::PrimitiveDateTime>,
    pub quarantined: bool,
    pub u_id: Uuid,
    pub u_version_id: Uuid,
    pub u_parent_id: Option<Uuid>,
    pub u_name: String,
    pub u_description: Option<String>,
    pub u_avatar: Option<Uuid>,
    pub u_banner: Option<Uuid>,
    pub u_bot: bool,
    pub u_system: bool,
    pub u_webhook_channel_id: Option<Uuid>,
    pub u_webhook_creator_id: Option<Uuid>,
    pub u_webhook_room_id: Option<Uuid>,
    pub u_registered_at: Option<time::PrimitiveDateTime>,
    pub u_deleted_at: Option<time::PrimitiveDateTime>,
}

// DbMediaLink and DbMediaPatch are used internally, not exported
