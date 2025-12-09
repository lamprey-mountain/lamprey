// TODO: rename foo_select to foo_get

use async_trait::async_trait;
use common::v1::types::application::{Application, Connection, Scope};
use common::v1::types::calendar::{
    CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventPatch,
};
use common::v1::types::email::{EmailAddr, EmailInfo, EmailInfoPatch};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch};
use common::v1::types::media::MediaWithAdmin;
use common::v1::types::notifications::{
    InboxListParams, Notification, NotificationFlush, NotificationMarkRead,
};
use common::v1::types::reaction::{ReactionKeyParam, ReactionListItem};
use common::v1::types::room_analytics::{
    RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
    RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
    RoomAnalyticsOverview, RoomAnalyticsParams,
};
use common::v1::types::search::{SearchChannelsRequest, SearchMessageRequest};
use common::v1::types::tag::{Tag, TagCreate, TagPatch};
use common::v1::types::user_config::{
    UserConfigChannel, UserConfigGlobal, UserConfigRoom, UserConfigUser,
};
use common::v1::types::util::Time;
use common::v1::types::webhook::{Webhook, WebhookCreate, WebhookUpdate};

use common::v1::types::{
    ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogFilter, CalendarEventId, Channel,
    ChannelId, ChannelPatch, ChannelReorder, ChannelVerId, Embed, EmojiId, InvitePatch,
    InviteWithMetadata, MediaPatch, NotificationId, PaginationQuery, PaginationResponse,
    Permission, PermissionOverwriteType, PinsReorder, Relationship, RelationshipPatch,
    RelationshipWithUserId, Role, RoleReorder, RoomBan, RoomMember, RoomMemberOrigin,
    RoomMemberPatch, RoomMemberPut, RoomMemberSearchAdvanced, RoomMemberSearchResponse,
    RoomMembership, SessionPatch, SessionStatus, SessionToken, Suspended, TagId, ThreadMember,
    ThreadMemberPut, ThreadMembership, UserListFilter, WebhookId,
};

use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbChannelCreate, DbChannelPrivate, DbEmailQueue, DbMessageCreate, DbRoleCreate, DbRoomCreate,
    DbSessionCreate, DbUserCreate, EmailPurpose, InviteCode, Media, MediaId, MediaLink,
    MediaLinkType, Message, MessageId, MessageRef, MessageVerId, Permissions, RoleId, RolePatch,
    RoleVerId, Room, RoomCreate, RoomId, RoomPatch, RoomVerId, Session, SessionId, UrlEmbedQueue,
    User, UserId, UserPatch, UserVerId,
};

pub mod postgres;

// #[async_trait]
pub trait Data:
    DataRoom
    + DataRoomMember
    + DataRole
    + DataRoleMember
    + DataPermission
    + DataInvite
    + DataMedia
    + DataMessage
    + DataSession
    + DataChannel
    + DataUnread
    + DataUser
    + DataSearch
    + DataAuth
    + DataAuditLogs
    + DataThreadMember
    + DataThread
    + DataUserRelationship
    + DataUserConfig
    + DataReaction
    + DataApplication
    + DataConnection
    + DataEmoji
    + DataCalendar
    + DataEmbed
    + DataUserEmail
    + DataEmailQueue
    + DataDm
    + DataNotification
    + DataWebhook
    + DataTag
    + DataMetrics
    + DataRoomAnalytics
    + Send
    + Sync
{
    // async fn commit(self) -> Result<()>;
    // async fn rollback(self) -> Result<()>;
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct InstanceMetrics {
    pub user_count_total: i64,
    pub user_count_guest: i64,
    pub user_count_registered: i64,
    pub user_count_bot: i64,
    pub user_count_webhook: i64,
    pub user_count_puppet: i64,
    pub user_count_puppet_bot: i64,
    pub room_count_total: i64,
    pub room_count_private: i64,
    pub room_count_public: i64,
    pub channel_count_total: i64,
    pub channel_count_text: i64,
    pub channel_count_voice: i64,
    pub channel_count_broadcast: i64,
    pub channel_count_calendar: i64,
    pub channel_count_thread_public: i64,
    pub channel_count_thread_private: i64,
    pub channel_count_dm: i64,
    pub channel_count_gdm: i64,
}

#[async_trait]
pub trait DataMetrics {
    async fn get_metrics(&self) -> Result<InstanceMetrics>;
}

#[async_trait]
pub trait DataRoom {
    async fn room_create(&self, create: RoomCreate, extra: DbRoomCreate) -> Result<Room>;
    async fn room_get(&self, room_id: RoomId) -> Result<Room>;
    async fn room_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomId>,
        include_server_room: bool,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_list_all(
        &self,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_list_mutual(
        &self,
        user_a_id: UserId,
        user_b_id: UserId,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_update(&self, room_id: RoomId, patch: RoomPatch) -> Result<RoomVerId>;
    async fn room_set_owner(&self, id: RoomId, owner_id: UserId) -> Result<RoomVerId>;
    async fn room_delete(&self, room_id: RoomId) -> Result<()>;
    async fn room_undelete(&self, room_id: RoomId) -> Result<()>;
    async fn room_quarantine(&self, room_id: RoomId) -> Result<RoomVerId>;
    async fn room_unquarantine(&self, room_id: RoomId) -> Result<RoomVerId>;
}

#[async_trait]
pub trait DataRoomMember {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        origin: Option<RoomMemberOrigin>,
        put: RoomMemberPut,
    ) -> Result<()>;
    async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomMemberPatch,
    ) -> Result<()>;
    async fn room_member_set_membership(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
    ) -> Result<()>;
    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()>;
    async fn room_member_get(&self, room_id: RoomId, user_id: UserId) -> Result<RoomMember>;
    async fn room_member_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>>;

    async fn room_member_list_all(&self, room_id: RoomId) -> Result<Vec<RoomMember>>;

    async fn room_member_search(
        &self,
        room_id: RoomId,
        query: String,
        limit: u16,
    ) -> Result<Vec<RoomMember>>;

    async fn room_member_search_advanced(
        &self,
        room_id: RoomId,
        search: RoomMemberSearchAdvanced,
    ) -> Result<RoomMemberSearchResponse>;

    async fn room_ban_create(
        &self,
        room_id: RoomId,
        ban_id: UserId,
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()>;
    async fn room_ban_delete(&self, room_id: RoomId, ban_id: UserId) -> Result<()>;
    async fn room_ban_get(&self, room_id: RoomId, ban_id: UserId) -> Result<RoomBan>;
    async fn room_ban_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>>;
    async fn room_ban_search(
        &self,
        room_id: RoomId,
        query: String,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>>;
    async fn room_ban_create_bulk(
        &self,
        room_id: RoomId,
        ban_ids: &[UserId],
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()>;

    async fn room_bot_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<ApplicationId>>;
}

#[async_trait]
pub trait DataTag {
    async fn tag_create(&self, forum_channel_id: ChannelId, create: TagCreate) -> Result<Tag>;
    async fn tag_update(&self, tag_id: TagId, patch: TagPatch) -> Result<Tag>;
    async fn tag_delete(&self, tag_id: TagId) -> Result<()>;
    async fn tag_get(&self, tag_id: TagId) -> Result<Tag>;
    async fn tag_get_forum_id(&self, tag_id: TagId) -> Result<ChannelId>;
}

#[async_trait]
pub trait DataRole {
    async fn role_create(&self, create: DbRoleCreate, position: u64) -> Result<Role>;
    // TODO: make this return all roles, paginate in server logic
    async fn role_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<RoleId>,
    ) -> Result<PaginationResponse<Role>>;
    async fn role_delete(&self, room_id: RoomId, role_id: RoleId) -> Result<()>;
    async fn role_select(&self, room_id: RoomId, role_id: RoleId) -> Result<Role>;
    async fn role_update(
        &self,
        room_id: RoomId,
        role_id: RoleId,
        patch: RolePatch,
    ) -> Result<RoleVerId>;
    async fn role_reorder(&self, room_id: RoomId, reorder: RoleReorder) -> Result<()>;
    async fn role_user_rank(&self, room_id: RoomId, user_id: UserId) -> Result<u64>;
}

#[async_trait]
pub trait DataRoleMember {
    async fn role_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        role_id: RoleId,
    ) -> Result<()>;
    async fn role_member_delete(
        &self,
        room_id: RoomId,
        user_id: UserId,
        role_id: RoleId,
    ) -> Result<()>;
    async fn role_member_list(
        &self,
        role_id: RoleId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>>;
    async fn role_member_count(&self, room_id: RoomId, role_id: RoleId) -> Result<u64>;
    async fn role_member_bulk_edit(
        &self,
        room_id: RoomId,
        role_id: RoleId,
        apply_user_ids: &[UserId],
        remove_user_ids: &[UserId],
    ) -> Result<()>;
}

#[async_trait]
pub trait DataPermission {
    async fn permission_room_get(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions>;
    async fn permission_is_mutual(&self, a: UserId, b: UserId) -> Result<bool>;
    async fn permission_overwrite_upsert(
        &self,
        channel_id: ChannelId,
        overwrite_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    ) -> Result<()>;
    async fn permission_overwrite_delete(
        &self,
        channel_id: ChannelId,
        overwrite_id: Uuid,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataInvite {
    async fn invite_select(&self, code: InviteCode) -> Result<InviteWithMetadata>;
    async fn invite_delete(&self, code: InviteCode) -> Result<()>;

    async fn invite_insert_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_room(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_server(
        &self,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_server(
        &self,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_list_server_by_creator(
        &self,
        creator_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_user(
        &self,
        user_id: UserId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_user(
        &self,
        user_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_channel(
        &self,
        channel_id: ChannelId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_channel(
        &self,
        channel_id: ChannelId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_incr_use(&self, code: InviteCode) -> Result<()>;
    async fn invite_update(
        &self,
        code: InviteCode,
        patch: InvitePatch,
    ) -> Result<InviteWithMetadata>;
}

#[async_trait]
pub trait DataMedia {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<()>;
    async fn media_select(&self, media_id: MediaId) -> Result<MediaWithAdmin>;
    async fn media_update(&self, media_id: MediaId, patch: MediaPatch) -> Result<()>;
    async fn media_delete(&self, media_id: MediaId) -> Result<()>;
    async fn media_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;
    async fn media_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>>;
    async fn media_link_delete(&self, target_id: Uuid, link_type: MediaLinkType) -> Result<()>;
    async fn media_link_delete_all(&self, target_id: Uuid) -> Result<()>;
    async fn media_link_create_exclusive(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataMessage {
    async fn message_create(&self, create: DbMessageCreate) -> Result<MessageId>;
    async fn message_update(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        create: DbMessageCreate,
    ) -> Result<MessageVerId>;
    async fn message_update_in_place(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
        create: DbMessageCreate,
    ) -> Result<()>;
    async fn message_get(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
    ) -> Result<Message>;
    async fn message_list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_deleted(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_removed(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_activity(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_delete(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_delete_bulk(
        &self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_remove_bulk(
        &self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_restore_bulk(
        &self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_version_get(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
        user_id: UserId,
    ) -> Result<Message>;
    async fn message_version_delete(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<()>;
    async fn message_version_list(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_replies(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_pin_create(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_pin_delete(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_pin_reorder(&self, channel_id: ChannelId, reorder: PinsReorder) -> Result<()>;
    async fn message_pin_list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
}

#[async_trait]
pub trait DataSession {
    async fn session_create(&self, create: DbSessionCreate) -> Result<Session>;
    async fn session_get(&self, session_id: SessionId) -> Result<Session>;
    async fn session_get_by_token(&self, token: SessionToken) -> Result<Session>;
    async fn session_set_status(&self, session_id: SessionId, status: SessionStatus) -> Result<()>;
    async fn session_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>>;
    async fn session_update(&self, session_id: SessionId, patch: SessionPatch) -> Result<()>;
    async fn session_delete(&self, session_id: SessionId) -> Result<()>;
    async fn session_delete_all(&self, user_id: UserId) -> Result<()>;
    async fn session_set_last_seen_at(&self, session_id: SessionId) -> Result<()>;
}

#[async_trait]
pub trait DataChannel {
    async fn channel_create(&self, create: DbChannelCreate) -> Result<ChannelId>;
    async fn channel_create_with_id(&self, id: ChannelId, create: DbChannelCreate) -> Result<()>;
    async fn channel_get(&self, channel_id: ChannelId) -> Result<Channel>;
    async fn channel_get_many(&self, channel_ids: &[ChannelId]) -> Result<Vec<Channel>>;
    async fn channel_get_private(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<DbChannelPrivate>;
    async fn channel_list(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>>;
    async fn channel_list_archived(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>>;
    async fn channel_list_removed(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>>;
    async fn channel_update(
        &self,
        channel_id: ChannelId,
        patch: ChannelPatch,
    ) -> Result<ChannelVerId>;
    async fn channel_delete(&self, channel_id: ChannelId) -> Result<()>;
    async fn channel_undelete(&self, channel_id: ChannelId) -> Result<()>;
    async fn channel_reorder(&self, data: ChannelReorder) -> Result<()>;
    async fn channel_upgrade_gdm(&self, channel_id: ChannelId, room_id: RoomId) -> Result<()>;

    async fn channel_get_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>>;
    async fn channel_set_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()>;
    async fn channel_get_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>>;
    async fn channel_set_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataUnread {
    async fn unread_ack(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        mention_count: Option<u64>,
    ) -> Result<()>;
    async fn unread_put_all_in_room(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId, MessageVerId)>>;
    async fn unread_increment_mentions(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        count: u32,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataUser {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User>;
    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId>;
    async fn user_delete(&self, user_id: UserId) -> Result<()>;
    async fn user_undelete(&self, user_id: UserId) -> Result<()>;
    async fn user_get(&self, user_id: UserId) -> Result<User>;
    async fn user_get_many(&self, user_ids: &[UserId]) -> Result<Vec<User>>;
    async fn user_list(
        &self,
        pagination: PaginationQuery<UserId>,
        filter: Option<UserListFilter>,
    ) -> Result<PaginationResponse<User>>;
    async fn user_lookup_puppet(
        &self,
        owner_id: UserId,
        external_id: &str,
    ) -> Result<Option<UserId>>;
    async fn user_set_registered(
        &self,
        user_id: UserId,
        registered_at: Option<Time>,
        parent_invite: Option<String>,
    ) -> Result<UserVerId>;
    async fn user_suspended(
        &self,
        user_id: UserId,
        suspended: Option<Suspended>,
    ) -> Result<UserVerId>;
}

#[async_trait]
pub trait DataAuth {
    async fn auth_oauth_put(
        &self,
        provider: String,
        user_id: UserId,
        remote_id: String,
        can_auth: bool,
    ) -> Result<()>;
    async fn auth_oauth_get_all(&self, user_id: UserId) -> Result<Vec<String>>;
    async fn auth_oauth_get_remote(&self, provider: String, remote_id: String) -> Result<UserId>;
    async fn auth_oauth_delete(&self, provider: String, user_id: UserId) -> Result<()>;
    async fn auth_password_set(&self, user_id: UserId, hash: &[u8], salt: &[u8]) -> Result<()>;
    async fn auth_password_get(&self, user_id: UserId) -> Result<Option<(Vec<u8>, Vec<u8>)>>;
    async fn auth_password_delete(&self, user_id: UserId) -> Result<()>;
    async fn auth_email_create(
        &self,
        code: String,
        addr: EmailAddr,
        session_id: SessionId,
        purpose: EmailPurpose,
    ) -> Result<()>;
    async fn auth_email_use(&self, code: String) -> Result<(EmailAddr, SessionId, EmailPurpose)>;
    async fn oauth_auth_code_create(
        &self,
        code: String,
        application_id: ApplicationId,
        user_id: UserId,
        redirect_uri: String,
        scopes: Vec<Scope>,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> Result<()>;
    async fn oauth_auth_code_use(
        &self,
        code: String,
    ) -> Result<(
        ApplicationId,
        UserId,
        String,
        Vec<Scope>,
        Option<String>,
        Option<String>,
    )>;
    async fn oauth_refresh_token_create(&self, token: String, session_id: SessionId) -> Result<()>;
    async fn oauth_refresh_token_use(&self, token: String) -> Result<SessionId>;
}

#[async_trait]
pub trait DataSearch {
    async fn search_message(
        &self,
        user_id: UserId,
        query: SearchMessageRequest,
        paginate: PaginationQuery<MessageId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Message>>;
    async fn search_channel(
        &self,
        user_id: UserId,
        query: SearchChannelsRequest,
        paginate: PaginationQuery<ChannelId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Channel>>;
}

#[async_trait]
pub trait DataAuditLogs {
    async fn audit_logs_room_fetch(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogEntryId>,
        filter: AuditLogFilter,
    ) -> Result<PaginationResponse<AuditLogEntry>>;
    async fn audit_logs_room_append(&self, entry: AuditLogEntry) -> Result<()>;
}

#[async_trait]
pub trait DataThreadMember {
    /// is a no-op if membership won't change
    async fn thread_member_put(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        put: ThreadMemberPut,
    ) -> Result<()>;
    async fn thread_member_set_membership(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()>;
    async fn thread_member_delete(&self, thread_id: ChannelId, user_id: UserId) -> Result<()>;
    async fn thread_member_get(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<ThreadMember>;
    async fn thread_member_list(
        &self,
        thread_id: ChannelId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>>;
    async fn thread_member_list_all(&self, thread_id: ChannelId) -> Result<Vec<ThreadMember>>;
}

#[async_trait]
pub trait DataThread {
    // returns all public threads and private threads the user is in by default. include_all should return all threads and should be set for thread moderators.
    async fn thread_list_active(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;
    async fn thread_list_archived(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;
    async fn thread_list_removed(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;

    /// Archive threads that have been inactive beyond their auto-archive duration
    async fn thread_auto_archive(&self) -> Result<Vec<ChannelId>>;
}

#[async_trait]
pub trait DataUserRelationship {
    async fn user_relationship_put(
        &self,
        user_id: UserId,
        other_id: UserId,
        rel: Relationship,
    ) -> Result<()>;
    async fn user_relationship_edit(
        &self,
        user_id: UserId,
        other_id: UserId,
        patch: RelationshipPatch,
    ) -> Result<()>;
    async fn user_relationship_delete(&self, user_id: UserId, other_id: UserId) -> Result<()>;
    async fn user_relationship_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<Option<Relationship>>;

    /// paginate users who have relationship Block
    async fn user_relationship_list_blocked(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who have relationship Friend
    async fn user_relationship_list_friends(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who have relationship Incoming or Outgoing
    async fn user_relationship_list_pending(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who are currently ignored
    async fn user_relationship_list_ignored(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;
}

#[async_trait]
pub trait DataUserConfig {
    async fn user_config_set(&self, user_id: UserId, config: &UserConfigGlobal) -> Result<()>;
    async fn user_config_get(&self, user_id: UserId) -> Result<UserConfigGlobal>;
    async fn user_config_room_set(
        &self,
        user_id: UserId,
        room_id: RoomId,
        config: &UserConfigRoom,
    ) -> Result<()>;
    async fn user_config_room_get(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<UserConfigRoom>;
    async fn user_config_channel_set(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        config: &UserConfigChannel,
    ) -> Result<()>;
    async fn user_config_channel_get(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<UserConfigChannel>;
    async fn user_config_user_set(
        &self,
        user_id: UserId,
        other_id: UserId,
        config: &UserConfigUser,
    ) -> Result<()>;
    async fn user_config_user_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<UserConfigUser>;
}

#[async_trait]
pub trait DataReaction {
    async fn reaction_put(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_list(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>>;
    async fn reaction_delete(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_delete_key(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_delete_all(&self, channel_id: ChannelId, message_id: MessageId)
        -> Result<()>;
    // TODO: make this return type less terrible
    async fn reaction_fetch_all(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        messages: &[MessageId],
    ) -> Result<Vec<(MessageId, Vec<(ReactionKeyParam, u64, bool)>)>>;
}

#[async_trait]
pub trait DataApplication {
    async fn application_insert(&self, data: Application) -> Result<()>;
    async fn application_update(&self, data: Application) -> Result<()>;
    async fn application_delete(&self, id: ApplicationId) -> Result<()>;
    async fn application_get(&self, id: ApplicationId) -> Result<Application>;
    async fn application_list(
        &self,
        owner_id: UserId,
        q: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Application>>;
}

#[async_trait]
pub trait DataConnection {
    async fn connection_create(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
        scopes: Vec<Scope>,
    ) -> Result<()>;
    async fn connection_get(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> Result<Connection>;
    async fn connection_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Connection>>;
    async fn connection_delete(&self, user_id: UserId, application_id: ApplicationId)
        -> Result<()>;
}

#[async_trait]
pub trait DataEmoji {
    async fn emoji_create(
        &self,
        creator_id: UserId,
        room_id: RoomId,
        create: EmojiCustomCreate,
    ) -> Result<EmojiCustom>;
    async fn emoji_get(&self, emoji_id: EmojiId) -> Result<EmojiCustom>;
    async fn emoji_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>>;
    async fn emoji_update(&self, emoji_id: EmojiId, patch: EmojiCustomPatch) -> Result<()>;
    async fn emoji_delete(&self, emoji_id: EmojiId) -> Result<()>;
}

#[async_trait]
pub trait DataEmbed {
    async fn url_embed_queue_insert(
        &self,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        url: String,
    ) -> Result<Uuid>;
    async fn url_embed_queue_claim(&self) -> Result<Option<UrlEmbedQueue>>;
    async fn url_embed_queue_finish(&self, id: Uuid, embed: Option<&Embed>) -> Result<()>;
}

#[async_trait]
pub trait DataUserEmail {
    async fn user_email_add(
        &self,
        user_id: UserId,
        email: EmailInfo,
        max_user_emails: usize,
    ) -> Result<()>;
    async fn user_email_delete(&self, user_id: UserId, email_addr: EmailAddr) -> Result<()>;
    async fn user_email_list(&self, user_id: UserId) -> Result<Vec<EmailInfo>>;
    async fn user_email_lookup(&self, email_addr: &EmailAddr) -> Result<UserId>;

    /// check and delete a code, and update is_verified
    async fn user_email_verify_use(
        &self,
        user_id: UserId,
        email_addr: EmailAddr,
        code: String,
    ) -> Result<()>;

    /// create a code and update last_updated_at
    async fn user_email_verify_create(
        &self,
        user_id: UserId,
        email_addr: EmailAddr,
    ) -> Result<String>;

    async fn user_email_update(
        &self,
        user_id: UserId,
        email_addr: EmailAddr,
        patch: EmailInfoPatch,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataEmailQueue {
    async fn email_queue_insert(
        &self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid>;
    async fn email_queue_claim(&self) -> Result<Option<DbEmailQueue>>;
    async fn email_queue_finish(&self, id: Uuid) -> Result<()>;
    async fn email_queue_fail(&self, error_message: String, id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait DataDm {
    async fn dm_put(
        &self,
        user_a_id: UserId,
        user_b_id: UserId,
        channel_id: ChannelId,
    ) -> Result<()>;
    async fn dm_get(&self, user_a_id: UserId, user_b_id: UserId) -> Result<Option<ChannelId>>;
    async fn dm_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Channel>>;
}

#[async_trait]
pub trait DataNotification {
    async fn notification_add(&self, user_id: UserId, notif: Notification) -> Result<()>;
    async fn notification_delete(&self, user_id: UserId, notif: NotificationId) -> Result<()>;
    async fn notification_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<NotificationId>,
        params: InboxListParams,
    ) -> Result<PaginationResponse<Notification>>;
    async fn notification_mark_read(
        &self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()>;
    async fn notification_mark_unread(
        &self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()>;
    async fn notification_flush(&self, user_id: UserId, params: NotificationFlush) -> Result<()>;
}

#[async_trait]
pub trait DataCalendar {
    async fn calendar_event_create(
        &self,
        create: CalendarEventCreate,
        channel_id: ChannelId,
        creator_id: UserId,
    ) -> Result<CalendarEvent>;
    async fn calendar_event_get(&self, event_id: CalendarEventId) -> Result<CalendarEvent>;
    async fn calendar_event_list(
        &self,
        channel_id: ChannelId,
        query: CalendarEventListQuery,
    ) -> Result<PaginationResponse<CalendarEvent>>;
    async fn calendar_event_update(
        &self,
        event_id: CalendarEventId,
        patch: CalendarEventPatch,
    ) -> Result<CalendarEvent>;
    async fn calendar_event_delete(&self, event_id: CalendarEventId) -> Result<()>;
    async fn calendar_event_rsvp_put(
        &self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_event_rsvp_delete(
        &self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_event_rsvp_list(&self, event_id: CalendarEventId) -> Result<Vec<UserId>>;
}

#[async_trait]
pub trait DataWebhook {
    async fn webhook_create(
        &self,
        channel_id: ChannelId,
        creator_id: UserId,
        create: WebhookCreate,
    ) -> Result<Webhook>;
    async fn webhook_get(&self, webhook_id: WebhookId) -> Result<Webhook>;
    async fn webhook_get_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<Webhook>;
    async fn webhook_list_channel(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>>;
    async fn webhook_list_room(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>>;
    async fn webhook_update(&self, webhook_id: WebhookId, patch: WebhookUpdate) -> Result<Webhook>;
    async fn webhook_update_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
        patch: WebhookUpdate,
    ) -> Result<Webhook>;
    async fn webhook_delete(&self, webhook_id: WebhookId) -> Result<()>;
    async fn webhook_delete_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<()>;
}

#[async_trait]
pub trait DataRoomAnalytics {
    async fn room_analytics_members_count(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>>;

    async fn room_analytics_members_join(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>>;

    async fn room_analytics_members_leave(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>>;

    async fn room_analytics_channels(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
        q2: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>>;

    async fn room_analytics_overview(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>>;

    async fn room_analytics_invites(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsInvites>>;

    async fn room_analytics_snapshot_all(&self) -> Result<()>;
    async fn room_analytics_get_last_snapshot_ts(&self) -> Result<Option<time::PrimitiveDateTime>>;
}
