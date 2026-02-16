// TODO: rename foo_select to foo_get

use async_trait::async_trait;
use common::v1::types::ack::AckBulkItem;
use common::v1::types::application::{Application, Connection, Scopes};
use common::v1::types::automod::{AutomodRule, AutomodRuleCreate, AutomodRuleUpdate};
use common::v1::types::calendar::{
    Calendar, CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventParticipant,
    CalendarEventParticipantQuery, CalendarEventPatch, CalendarOverwrite, CalendarOverwritePut,
    CalendarPatch,
};
use common::v1::types::document::{
    Document, DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchPatch,
    DocumentBranchState, DocumentPatch, DocumentTag, Wiki, WikiPatch,
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
use common::v1::types::search::{ChannelSearchRequest, MessageSearchRequest};
use common::v1::types::tag::{Tag, TagCreate, TagPatch};
use common::v1::types::user_config::{
    PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser,
};
use common::v1::types::util::Time;
use common::v1::types::webhook::{Webhook, WebhookCreate, WebhookUpdate};

use common::v1::types::{
    ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogFilter, AutomodRuleId, CalendarEventId,
    Channel, ChannelId, ChannelPatch, ChannelReorder, ChannelVerId, DocumentBranchId,
    DocumentTagId, Embed, EmojiId, InvitePatch, InviteWithMetadata, MediaPatch, NotificationId,
    PaginationQuery, PaginationResponse, Permission, PermissionOverwriteType, PinsReorder,
    Relationship, RelationshipPatch, RelationshipWithUserId, Role, RoleReorder, RoomBan,
    RoomMember, RoomMemberOrigin, RoomMemberPatch, RoomMemberPut, RoomMemberSearchAdvanced,
    RoomMemberSearchResponse, SessionPatch, SessionStatus, SessionToken, Suspended, TagId,
    ThreadMember, ThreadMemberPut, UserListFilter, WebhookId,
};

use common::v2::types::message::{Message as MessageV2, MessageVersion as MessageVersionV2};

use uuid::Uuid;

use crate::error::Result;
use crate::types::admin::AdminCollectGarbageMode;
use crate::types::{
    DbChannelCreate, DbChannelPrivate, DbEmailQueue, DbMessageCreate, DbRoleCreate, DbRoomCreate,
    DbSessionCreate, DbUserCreate, DehydratedDocument, DocumentUpdateSummary, EmailPurpose,
    InviteCode, Media, MediaId, MediaLink, MediaLinkType, MentionsIds, MessageId, MessageRef,
    MessageVerId, PushData, RoleId, RolePatch, RoleVerId, Room, RoomCreate, RoomId, RoomPatch,
    RoomVerId, Session, SessionId, UrlEmbedQueue, User, UserId, UserPatch, UserVerId,
};
use crate::ConfigInternal;

pub type EditContextId = (ChannelId, DocumentBranchId);

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
    + DataSearchQueue
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
    + DataAdmin
    + DataAutomod
    + DataDocument
    + DataPush
    + DataConfigInternal
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
pub trait DataAutomod {
    async fn automod_rule_create(
        &self,
        room_id: RoomId,
        create: AutomodRuleCreate,
    ) -> Result<AutomodRule>;
    async fn automod_rule_get(&self, rule_id: AutomodRuleId) -> Result<AutomodRule>;
    async fn automod_rule_update(
        &self,
        rule_id: AutomodRuleId,
        update: AutomodRuleUpdate,
    ) -> Result<AutomodRule>;
    async fn automod_rule_delete(&self, rule_id: AutomodRuleId) -> Result<()>;
    async fn automod_rule_list(&self, room_id: RoomId) -> Result<Vec<AutomodRule>>;
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
    async fn user_room_count(&self, user_id: UserId) -> Result<u64>;
    async fn room_security_update(
        &self,
        room_id: RoomId,
        require_mfa: Option<bool>,
        require_sudo: Option<bool>,
    ) -> Result<RoomVerId>;
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
    async fn room_member_set_quarantined(
        &self,
        room_id: RoomId,
        user_id: UserId,
        quarantined: bool,
    ) -> Result<()>;

    /// soft delete a room member
    async fn room_member_leave(&self, room_id: RoomId, user_id: UserId) -> Result<()>;

    // NOTE: this is unused. consider removing it?
    // i might want some kind of way to prune room members
    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()>;

    async fn room_member_get(&self, room_id: RoomId, user_id: UserId) -> Result<RoomMember>;
    async fn room_member_get_many(
        &self,
        room_id: RoomId,
        user_ids: &[UserId],
    ) -> Result<Vec<RoomMember>>;
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
    async fn tag_search(
        &self,
        forum_channel_id: ChannelId,
        query: String,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>>;
    async fn tag_list(
        &self,
        forum_channel_id: ChannelId,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>>;
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
    async fn role_get_many(&self, room_id: RoomId, role_ids: &[RoleId]) -> Result<Vec<Role>>;
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
        role_ids: &[RoleId],
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
        role_ids: &[RoleId],
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
    ) -> Result<MessageV2>;
    async fn message_get_many(
        &self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
        user_id: UserId,
    ) -> Result<Vec<MessageV2>>;
    async fn message_list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_list_deleted(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_list_removed(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_list_activity(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_list_all(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
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
    ) -> Result<MessageVersionV2>;
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
    ) -> Result<PaginationResponse<MessageVersionV2>>;
    async fn message_replies(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_pin_create(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_pin_delete(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_pin_reorder(&self, channel_id: ChannelId, reorder: PinsReorder) -> Result<()>;
    async fn message_pin_list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn message_get_ancestors(
        &self,
        message_id: MessageId,
        limit: u16,
    ) -> Result<Vec<MessageV2>>;
    async fn message_fetch_mention_ids(
        &self,
        channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<Vec<MentionsIds>>;
    async fn message_id_get_by_version(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageId>;
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
    // TODO: replace session_set_last_seen_at with session_heartbeat
    async fn session_set_last_seen_at(&self, session_id: SessionId) -> Result<()>;
    // /// update last seen at and other metadata
    // async fn session_heartbeat(&self, session_id: SessionId, ip_addr: IpAddr, user_agent: String) -> Result<()>;
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

    /// list all (non-thread) channels in this room that have been removed
    async fn channel_list(&self, room_id: RoomId) -> Result<Vec<Channel>>;
    async fn channel_list_removed(
        &self,
        room_id: RoomId,
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

    async fn channel_document_insert(
        &self,
        channel_id: ChannelId,
        document: &Document,
    ) -> Result<()>;
    async fn channel_document_get(&self, channel_id: ChannelId) -> Result<Option<Document>>;
    async fn channel_document_update(
        &self,
        channel_id: ChannelId,
        document_patch: &DocumentPatch,
    ) -> Result<()>;

    async fn channel_wiki_insert(&self, channel_id: ChannelId, wiki: &Wiki) -> Result<()>;
    async fn channel_wiki_get(&self, channel_id: ChannelId) -> Result<Option<Wiki>>;
    async fn channel_wiki_update(
        &self,
        channel_id: ChannelId,
        wiki_patch: &WikiPatch,
    ) -> Result<()>;

    async fn channel_calendar_insert(
        &self,
        channel_id: ChannelId,
        calendar: &Calendar,
    ) -> Result<()>;
    async fn channel_calendar_get(&self, channel_id: ChannelId) -> Result<Option<Calendar>>;
    async fn channel_calendar_update(
        &self,
        channel_id: ChannelId,
        calendar_patch: &CalendarPatch,
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
    async fn unread_ack_bulk(&self, user_id: UserId, acks: Vec<AckBulkItem>) -> Result<()>;
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
    async fn auth_totp_set(
        &self,
        user_id: UserId,
        secret: Option<String>,
        enabled: bool,
    ) -> Result<()>;
    async fn auth_totp_get(&self, user_id: UserId) -> Result<Option<(String, bool)>>;
    async fn auth_totp_recovery_generate(&self, user_id: UserId, codes: &[String]) -> Result<()>;
    async fn auth_totp_recovery_get_all(
        &self,
        user_id: UserId,
    ) -> Result<Vec<(String, Option<Time>)>>;
    async fn auth_totp_recovery_use(&self, user_id: UserId, code: &str) -> Result<()>;
    async fn auth_totp_recovery_delete_all(&self, user_id: UserId) -> Result<()>;

    // TODO: move these into a new DataOauth trait?
    async fn oauth_auth_code_create(
        &self,
        code: String,
        application_id: ApplicationId,
        user_id: UserId,
        redirect_uri: String,
        scopes: Scopes,
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
        Scopes,
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
        query: MessageSearchRequest,
        paginate: PaginationQuery<MessageId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn search_channel(
        &self,
        user_id: UserId,
        query: ChannelSearchRequest,
        paginate: PaginationQuery<ChannelId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Channel>>;
}

#[async_trait]
pub trait DataSearchQueue {
    async fn search_reindex_queue_upsert(
        &self,
        channel_id: ChannelId,
        last_message_id: Option<MessageId>,
    ) -> Result<()>;
    async fn search_reindex_queue_list(
        &self,
        limit: u32,
    ) -> Result<Vec<(ChannelId, Option<MessageId>)>>;
    async fn search_reindex_queue_delete(&self, channel_id: ChannelId) -> Result<()>;
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
    async fn thread_member_leave(&self, thread_id: ChannelId, user_id: UserId) -> Result<()>;
    async fn thread_member_delete(&self, thread_id: ChannelId, user_id: UserId) -> Result<()>;
    async fn thread_member_get(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<ThreadMember>;
    async fn thread_member_get_many(
        &self,
        thread_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<Vec<ThreadMember>>;
    async fn thread_member_list(
        &self,
        thread_id: ChannelId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>>;
    async fn thread_member_list_all(&self, thread_id: ChannelId) -> Result<Vec<ThreadMember>>;

    /// fetch thread member object for all of these threads
    async fn thread_member_bulk_fetch(
        &self,
        user_id: UserId,
        thread_ids: &[ChannelId],
    ) -> Result<Vec<(ChannelId, ThreadMember)>>;
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

    /// archive threads that have been inactive beyond their auto-archive duration
    async fn thread_auto_archive(&self) -> Result<Vec<ChannelId>>;

    /// list all active threads in a room
    async fn thread_all_active_room(&self, room_id: RoomId) -> Result<Vec<Channel>>;
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
    async fn user_config_set(&self, user_id: UserId, config: &PreferencesGlobal) -> Result<()>;
    async fn user_config_get(&self, user_id: UserId) -> Result<PreferencesGlobal>;
    async fn user_config_room_set(
        &self,
        user_id: UserId,
        room_id: RoomId,
        config: &PreferencesRoom,
    ) -> Result<()>;
    async fn user_config_room_get(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<PreferencesRoom>;
    async fn user_config_channel_set(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        config: &PreferencesChannel,
    ) -> Result<()>;
    async fn user_config_channel_get(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<PreferencesChannel>;
    async fn user_config_user_set(
        &self,
        user_id: UserId,
        other_id: UserId,
        config: &PreferencesUser,
    ) -> Result<()>;
    async fn user_config_user_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<PreferencesUser>;
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
        scopes: Scopes,
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
    async fn emoji_search(
        &self,
        user_id: UserId,
        query: String,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>>;
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
    async fn notification_get_unpushed(&self, limit: u32) -> Result<Vec<(UserId, Notification)>>;
    async fn notification_set_pushed(&self, ids: &[NotificationId]) -> Result<()>;
}

#[async_trait]
pub trait DataConfigInternal {
    async fn config_put(&self, config: ConfigInternal) -> Result<()>;
    async fn config_get(&self) -> Result<Option<ConfigInternal>>;
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

    // RSVP methods for the event (series)
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
    async fn calendar_event_rsvp_get(
        &self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<CalendarEventParticipant>;
    async fn calendar_event_rsvp_list(
        &self,
        event_id: CalendarEventId,
        query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>>;

    // Overwrite methods
    async fn calendar_overwrite_put(
        &self,
        event_id: CalendarEventId,
        seq: u64,
        put: CalendarOverwritePut,
    ) -> Result<CalendarOverwrite>;
    async fn calendar_overwrite_get(
        &self,
        event_id: CalendarEventId,
        seq: u64,
    ) -> Result<CalendarOverwrite>;
    async fn calendar_overwrite_list(
        &self,
        event_id: CalendarEventId,
    ) -> Result<Vec<CalendarOverwrite>>;
    async fn calendar_overwrite_delete(&self, event_id: CalendarEventId, seq: u64) -> Result<()>;

    // RSVP methods for overwrites
    async fn calendar_overwrite_rsvp_put(
        &self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
        attending: bool,
    ) -> Result<()>;
    async fn calendar_overwrite_rsvp_delete(
        &self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_overwrite_rsvp_list(
        &self,
        event_id: CalendarEventId,
        seq: u64,
        query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>>;
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

#[async_trait]
pub trait DataAdmin {
    /// garbage collect room analytics data
    ///
    /// returns rows affected
    async fn gc_room_analytics(&self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// garbage collect messages data
    ///
    /// returns rows affected
    async fn gc_messages(&self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// marks media for garbage collection
    async fn gc_media_mark(&self) -> Result<u64>;

    /// gets candidates for media garbage collection sweep
    async fn gc_media_get_sweep_candidates(&self, limit: u32) -> Result<Vec<MediaId>>;

    /// deletes media that has been swept
    async fn gc_media_delete_swept(&self, ids: &[MediaId]) -> Result<u64>;

    /// counts media marked for deletion
    async fn gc_media_count_deleted(&self) -> Result<u64>;

    /// garbage collect sessions
    ///
    /// returns rows affected
    async fn gc_sessions(&self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// garbage collect audit logs
    ///
    /// returns rows affected
    async fn gc_audit_logs(&self, mode: AdminCollectGarbageMode) -> Result<u64>;
}

#[async_trait]
pub trait DataDocument {
    /// save a new snapshot
    async fn document_compact(
        &self,
        context_id: EditContextId,
        last_snapshot_id: Uuid,
        last_seq: u32,
        snapshot: Vec<u8>,
    ) -> Result<()>;

    /// loads the latest snapshot of a document, along with the last changes applied to it
    async fn document_load(&self, context_id: EditContextId) -> Result<DehydratedDocument>;

    /// attempts to create a new document if it doesnt already exist (create default branch, create initial snapshot)
    async fn document_create(
        &self,
        context_id: EditContextId,
        creator_id: UserId,
        snapshot: Vec<u8>,
    ) -> Result<()>;

    /// save an update. uses latest snapshot_id and increments seq. returns the update's seq number.
    async fn document_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        update: Vec<u8>,
        stat_added: u32,
        stat_removed: u32,
    ) -> Result<u32>;

    /// create a new branch
    async fn document_fork(
        &self,
        context_id: EditContextId,
        creator_id: UserId,
        create: DocumentBranchCreate,
    ) -> Result<DocumentBranchId>;

    /// get a branch
    async fn document_branch_get(
        &self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
    ) -> Result<DocumentBranch>;

    /// update a branch
    async fn document_branch_update(
        &self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
        patch: DocumentBranchPatch,
    ) -> Result<()>;

    /// set a branch's state
    async fn document_branch_set_state(
        &self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
        status: DocumentBranchState,
    ) -> Result<()>;

    /// list all active branches
    async fn document_branch_list(&self, document_id: ChannelId) -> Result<Vec<DocumentBranch>>;

    /// paginate through branches
    async fn document_branch_paginate(
        &self,
        document_id: ChannelId,
        user_id: UserId,
        filter: DocumentBranchListParams,
        pagination: PaginationQuery<DocumentBranchId>,
    ) -> Result<PaginationResponse<DocumentBranch>>;

    /// create a document tag
    async fn document_tag_create(
        &self,
        branch_id: DocumentBranchId,
        creator_id: UserId,
        summary: String,
        description: Option<String>,
        revision_seq: u64,
    ) -> Result<DocumentTagId>;

    /// get a document tag
    async fn document_tag_get(&self, tag_id: DocumentTagId) -> Result<DocumentTag>;

    /// update a document tag
    async fn document_tag_update(
        &self,
        tag_id: DocumentTagId,
        summary: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<()>;

    /// delete a document tag
    async fn document_tag_delete(&self, tag_id: DocumentTagId) -> Result<()>;

    /// list document tags for a branch
    async fn document_tag_list(&self, branch_id: DocumentBranchId) -> Result<Vec<DocumentTag>>;

    /// list document tags for a document (all branches)
    async fn document_tag_list_by_document(
        &self,
        document_id: ChannelId,
        user_id: UserId,
    ) -> Result<Vec<DocumentTag>>;

    /// fetch history for a document
    // TEMP: fetch ALL changes and tags for a document; this will be optimized later
    async fn document_history(
        &self,
        context_id: EditContextId,
    ) -> Result<(Vec<DocumentUpdateSummary>, Vec<DocumentTag>)>;

    /// fetch history for a wiki
    async fn wiki_history(
        &self,
        wiki_id: ChannelId,
    ) -> Result<(Vec<DocumentUpdateSummary>, Vec<DocumentTag>)>;
}

#[async_trait]
pub trait DataPush {
    /// insert a web push api subscription
    async fn push_insert(&self, push: PushData) -> Result<()>;

    /// get a web push api subscription for a session
    async fn push_get(&self, session_id: SessionId) -> Result<PushData>;

    /// delete a web push api subscription for a session
    async fn push_delete(&self, session_id: SessionId) -> Result<()>;

    /// list all web push subscriptions for a user
    async fn push_list_for_user(&self, user_id: UserId) -> Result<Vec<PushData>>;

    /// delete all web push subscriptions for a user
    async fn push_delete_for_user(&self, user_id: UserId) -> Result<()>;
}
