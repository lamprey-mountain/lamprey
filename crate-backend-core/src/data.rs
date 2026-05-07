use async_trait::async_trait;
use common::v1::types::Message as MessageV2;
use common::v1::types::{
    ack::AckBulkItem,
    application::{Application, Connection, Scopes},
    automod::{AutomodRule, AutomodRuleCreate, AutomodRuleUpdate},
    calendar::{
        CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventParticipant,
        CalendarEventParticipantQuery, CalendarEventPatch, CalendarOverwrite, CalendarOverwritePut,
    },
    email::{EmailAddr, EmailInfo, EmailInfoPatch},
    emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch},
    notifications::{InboxListParams, Notification, NotificationFlush, NotificationMarkRead},
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    reaction::{ReactionKeyParam, ReactionListItem},
    room_analytics::{
        RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
        RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    search::{ChannelSearchRequest, MessageSearchRequest},
    tag::{Tag, TagCreate, TagPatch},
    util::Time,
    webhook::{Webhook, WebhookCreate, WebhookUpdate},
    ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogFilter, AutomodRuleId, CalendarEventId,
    Channel, ChannelId, EmojiId, InviteCode, InvitePatch, InviteWithMetadata, MediaId, MessageId,
    MessageVerId, NotificationId, PaginationQuery, PaginationResponse, Permission,
    PermissionOverwriteType, Relationship, RelationshipPatch, RelationshipWithUserId, RoleId,
    RoomBan, RoomId, RoomMember, RoomMemberOrigin, RoomMemberPatch, RoomMemberPut,
    RoomMemberSearchAdvanced, RoomMemberSearchResponse, SearchDlqId, TagId, ThreadMember,
    ThreadMemberPut, UserId, WebhookId,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{config::ConfigInternal, types::admin::AdminCollectGarbageMode, Result};

#[async_trait]
pub trait DataRoleMember {
    async fn role_member_put(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
        role_id: RoleId,
    ) -> Result<()>;
    async fn role_member_delete(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
        role_id: RoleId,
    ) -> Result<()>;
    async fn role_member_list(
        &mut self,
        role_id: RoleId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>>;
    async fn role_member_count(&mut self, room_id: RoomId, role_id: RoleId) -> Result<u64>;
    async fn role_member_bulk_edit(
        &mut self,
        room_id: RoomId,
        role_id: RoleId,
        apply_user_ids: &[UserId],
        remove_user_ids: &[UserId],
    ) -> Result<()>;
}

#[async_trait]
pub trait DataRoomAnalytics {
    async fn room_analytics_members_count(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>>;

    async fn room_analytics_members_join(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>>;

    async fn room_analytics_members_leave(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>>;

    async fn room_analytics_channels(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
        q2: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>>;

    async fn room_analytics_overview(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>>;

    async fn room_analytics_invites(
        &mut self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsInvites>>;

    async fn room_analytics_snapshot_all(&mut self) -> Result<()>;
    async fn room_analytics_get_last_snapshot_ts(
        &mut self,
    ) -> Result<Option<time::PrimitiveDateTime>>;
}

#[async_trait]
pub trait DataWebhook {
    async fn webhook_create(
        &mut self,
        channel_id: ChannelId,
        creator_id: UserId,
        create: WebhookCreate,
    ) -> Result<Webhook>;
    async fn webhook_get(&mut self, webhook_id: WebhookId) -> Result<Webhook>;
    async fn webhook_get_with_token(
        &mut self,
        webhook_id: WebhookId,
        token: &str,
    ) -> Result<Webhook>;
    async fn webhook_list_channel(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>>;
    async fn webhook_list_room(
        &mut self,
        room_id: RoomId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>>;
    async fn webhook_update(
        &mut self,
        webhook_id: WebhookId,
        patch: WebhookUpdate,
    ) -> Result<Webhook>;
    async fn webhook_update_with_token(
        &mut self,
        webhook_id: WebhookId,
        token: &str,
        patch: WebhookUpdate,
    ) -> Result<Webhook>;
    async fn webhook_delete(&mut self, webhook_id: WebhookId) -> Result<()>;
    async fn webhook_delete_with_token(&mut self, webhook_id: WebhookId, token: &str)
        -> Result<()>;
}

#[async_trait]
pub trait DataAutomod {
    async fn automod_rule_create(
        &mut self,
        room_id: RoomId,
        create: AutomodRuleCreate,
    ) -> Result<AutomodRule>;
    async fn automod_rule_get(&mut self, rule_id: AutomodRuleId) -> Result<AutomodRule>;
    async fn automod_rule_update(
        &mut self,
        rule_id: AutomodRuleId,
        update: AutomodRuleUpdate,
    ) -> Result<AutomodRule>;
    async fn automod_rule_delete(&mut self, rule_id: AutomodRuleId) -> Result<()>;
    async fn automod_rule_list(&mut self, room_id: RoomId) -> Result<Vec<AutomodRule>>;
}

#[async_trait]
pub trait DataRoomMember {
    async fn room_member_put(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
        origin: Option<RoomMemberOrigin>,
        put: RoomMemberPut,
    ) -> Result<()>;
    async fn room_member_patch(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomMemberPatch,
    ) -> Result<()>;
    async fn room_member_set_quarantined(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
        quarantined: bool,
    ) -> Result<()>;

    /// soft delete a room member
    async fn room_member_leave(&mut self, room_id: RoomId, user_id: UserId) -> Result<()>;

    // NOTE: this is unused. consider removing it?
    // i might want some kind of way to prune room members
    async fn room_member_delete(&mut self, room_id: RoomId, user_id: UserId) -> Result<()>;

    async fn room_member_get(&mut self, room_id: RoomId, user_id: UserId) -> Result<RoomMember>;
    async fn room_member_get_many(
        &mut self,
        room_id: RoomId,
        user_ids: &[UserId],
    ) -> Result<Vec<RoomMember>>;
    async fn room_member_list(
        &mut self,
        room_id: RoomId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>>;

    async fn room_member_list_all(&mut self, room_id: RoomId) -> Result<Vec<RoomMember>>;

    async fn room_member_search(
        &mut self,
        room_id: RoomId,
        query: String,
        limit: u16,
    ) -> Result<Vec<RoomMember>>;

    async fn room_member_search_advanced(
        &mut self,
        room_id: RoomId,
        search: RoomMemberSearchAdvanced,
    ) -> Result<RoomMemberSearchResponse>;

    async fn room_ban_create(
        &mut self,
        room_id: RoomId,
        ban_id: UserId,
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()>;
    async fn room_ban_delete(&mut self, room_id: RoomId, ban_id: UserId) -> Result<()>;
    async fn room_ban_get(&mut self, room_id: RoomId, ban_id: UserId) -> Result<RoomBan>;
    async fn room_ban_list(
        &mut self,
        room_id: RoomId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>>;
    async fn room_ban_search(
        &mut self,
        room_id: RoomId,
        query: String,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>>;
    async fn room_ban_create_bulk(
        &mut self,
        room_id: RoomId,
        ban_ids: &[UserId],
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()>;

    async fn room_bot_list(
        &mut self,
        room_id: RoomId,
        paginate: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<ApplicationId>>;
}

#[async_trait]
pub trait DataTag {
    async fn tag_create(&mut self, forum_channel_id: ChannelId, create: TagCreate) -> Result<Tag>;
    async fn tag_update(&mut self, tag_id: TagId, patch: TagPatch) -> Result<Tag>;
    async fn tag_delete(&mut self, tag_id: TagId) -> Result<()>;
    async fn tag_get(&mut self, tag_id: TagId) -> Result<Tag>;
    async fn tag_get_forum_id(&mut self, tag_id: TagId) -> Result<ChannelId>;
    async fn tag_search(
        &mut self,
        forum_channel_id: ChannelId,
        query: String,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>>;
    async fn tag_list(
        &mut self,
        forum_channel_id: ChannelId,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>>;
}

#[async_trait]
pub trait DataPermission {
    async fn permission_is_mutual(&mut self, a: UserId, b: UserId) -> Result<bool>;
    async fn permission_overwrite_upsert(
        &mut self,
        channel_id: ChannelId,
        overwrite_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    ) -> Result<()>;
    async fn permission_overwrite_delete(
        &mut self,
        channel_id: ChannelId,
        overwrite_id: Uuid,
    ) -> Result<()>;

    /// Check if target user allows DMs from source user (via global or any shared room)
    async fn permission_allows_dm_from_user(
        &mut self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool>;

    /// Check if target user allows friend requests from source user (via global or any shared room)
    async fn permission_allows_friend_request_from_user(
        &mut self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool>;
}

#[async_trait]
pub trait DataUnread {
    async fn unread_ack(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        mention_count: Option<u64>,
    ) -> Result<()>;
    async fn unread_ack_bulk(&mut self, user_id: UserId, acks: Vec<AckBulkItem>) -> Result<()>;
    async fn unread_put_all_in_room(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId, MessageVerId)>>;
    async fn unread_increment_mentions(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        count: u32,
    ) -> Result<()>;
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
    async fn get_metrics(&mut self) -> Result<InstanceMetrics>;
}

#[async_trait]
pub trait DataInvite {
    async fn invite_select(&mut self, code: InviteCode) -> Result<InviteWithMetadata>;
    async fn invite_delete(&mut self, code: InviteCode) -> Result<()>;

    async fn invite_insert_room(
        &mut self,
        room_id: RoomId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
        role_ids: &[RoleId],
    ) -> Result<()>;
    async fn invite_list_room(
        &mut self,
        room_id: RoomId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_server(
        &mut self,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_server(
        &mut self,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_list_server_by_creator(
        &mut self,
        creator_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_user(
        &mut self,
        user_id: UserId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()>;
    async fn invite_list_user(
        &mut self,
        user_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_insert_channel(
        &mut self,
        channel_id: ChannelId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
        role_ids: &[RoleId],
    ) -> Result<()>;
    async fn invite_list_channel(
        &mut self,
        channel_id: ChannelId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    async fn invite_incr_use(&mut self, code: InviteCode) -> Result<()>;
    async fn invite_update(
        &mut self,
        code: InviteCode,
        patch: InvitePatch,
    ) -> Result<InviteWithMetadata>;
}

#[async_trait]
pub trait DataSearch {
    async fn search_message(
        &mut self,
        user_id: UserId,
        query: MessageSearchRequest,
        paginate: PaginationQuery<MessageId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<MessageV2>>;
    async fn search_channel(
        &mut self,
        user_id: UserId,
        query: ChannelSearchRequest,
        paginate: PaginationQuery<ChannelId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Channel>>;
}

#[async_trait]
pub trait DataSearchQueue {
    async fn search_reindex_queue_upsert(
        &mut self,
        target_type: &str,
        target_id: Uuid,
        last_id: Option<Uuid>,
    ) -> Result<()>;
    async fn search_reindex_queue_list(
        &mut self,
        target_type: &str,
        limit: u32,
    ) -> Result<Vec<(Uuid, Option<Uuid>)>>;
    async fn search_reindex_queue_delete(
        &mut self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<()>;
    async fn search_reindex_queue_get(
        &mut self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<Uuid>>;
    async fn search_reindex_queue_upsert_room(&mut self, room_id: RoomId) -> Result<()>;
    async fn search_reindex_queue_upsert_all(&mut self) -> Result<()>;

    async fn search_ingestion_dlq_insert(
        &mut self,
        entity_id: Uuid,
        entity_type: &str,
        error_message: &str,
    ) -> Result<()>;
    async fn search_ingestion_dlq_list(
        &mut self,
        pagination: PaginationQuery<SearchDlqId>,
    ) -> Result<PaginationResponse<crate::types::admin::DlqEntry>>;
    async fn search_ingestion_dlq_delete(&mut self, id: SearchDlqId) -> Result<()>;
}

#[async_trait]
pub trait DataAuditLogs {
    async fn audit_logs_room_fetch(
        &mut self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogEntryId>,
        filter: AuditLogFilter,
    ) -> Result<PaginationResponse<AuditLogEntry>>;
    async fn audit_logs_room_append(&mut self, entry: AuditLogEntry) -> Result<()>;
}

#[async_trait]
pub trait DataThreadMember {
    /// is a no-op if membership won't change
    async fn thread_member_put(
        &mut self,
        thread_id: ChannelId,
        user_id: UserId,
        put: ThreadMemberPut,
    ) -> Result<()>;
    async fn thread_member_leave(&mut self, thread_id: ChannelId, user_id: UserId) -> Result<()>;
    async fn thread_member_delete(&mut self, thread_id: ChannelId, user_id: UserId) -> Result<()>;
    async fn thread_member_get(
        &mut self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<ThreadMember>;
    async fn thread_member_get_many(
        &mut self,
        thread_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<Vec<ThreadMember>>;
    async fn thread_member_list(
        &mut self,
        thread_id: ChannelId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>>;
    async fn thread_member_list_all(&mut self, thread_id: ChannelId) -> Result<Vec<ThreadMember>>;

    /// fetch thread member object for all of these threads
    async fn thread_member_bulk_fetch(
        &mut self,
        user_id: UserId,
        thread_ids: &[ChannelId],
    ) -> Result<Vec<(ChannelId, ThreadMember)>>;
}

#[async_trait]
pub trait DataThread {
    // returns all public threads and private threads the user is in by default. include_all should return all threads and should be set for thread moderators.
    async fn thread_list_active(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;
    async fn thread_list_archived(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;
    async fn thread_list_removed(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>>;

    /// archive threads that have been inactive beyond their auto-archive duration
    async fn thread_auto_archive(&mut self) -> Result<Vec<ChannelId>>;

    /// list all active threads in a room
    async fn thread_all_active_room(&mut self, room_id: RoomId) -> Result<Vec<Channel>>;
}

#[async_trait]
pub trait DataUserRelationship {
    async fn user_relationship_put(
        &mut self,
        user_id: UserId,
        other_id: UserId,
        rel: Relationship,
    ) -> Result<()>;
    async fn user_relationship_edit(
        &mut self,
        user_id: UserId,
        other_id: UserId,
        patch: RelationshipPatch,
    ) -> Result<()>;
    async fn user_relationship_delete(&mut self, user_id: UserId, other_id: UserId) -> Result<()>;
    async fn user_relationship_get(
        &mut self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<Option<Relationship>>;

    /// paginate users who have relationship Block
    async fn user_relationship_list_blocked(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who have relationship Friend
    async fn user_relationship_list_friends(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who have relationship Incoming or Outgoing
    async fn user_relationship_list_pending(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// paginate users who are currently ignored
    async fn user_relationship_list_ignored(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;

    /// check if two users share a room (both are members)
    async fn user_shares_room(&mut self, user_a: UserId, user_b: UserId) -> Result<bool>;

    /// get shared room IDs between two users (both are members)
    async fn user_shared_rooms(&mut self, user_a: UserId, user_b: UserId) -> Result<Vec<RoomId>>;

    /// check if two users have at least one mutual friend
    async fn user_has_mutual_friend(&mut self, user_a: UserId, user_b: UserId) -> Result<bool>;
}

#[async_trait]
pub trait DataPreferences {
    async fn preferences_set(&mut self, user_id: UserId, config: &PreferencesGlobal) -> Result<()>;
    async fn preferences_get(&mut self, user_id: UserId) -> Result<PreferencesGlobal>;
    async fn preferences_room_set(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
        config: &PreferencesRoom,
    ) -> Result<()>;
    async fn preferences_room_get(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<PreferencesRoom>;
    async fn preferences_room_get_many(
        &mut self,
        user_id: UserId,
        room_ids: &[RoomId],
    ) -> Result<HashMap<RoomId, PreferencesRoom>>;
    async fn preferences_channel_set(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        config: &PreferencesChannel,
    ) -> Result<()>;
    async fn preferences_channel_get(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<PreferencesChannel>;
    async fn preferences_channel_get_many(
        &mut self,
        user_id: UserId,
        channel_ids: &[ChannelId],
    ) -> Result<HashMap<ChannelId, PreferencesChannel>>;
    async fn preferences_user_set(
        &mut self,
        user_id: UserId,
        other_id: UserId,
        config: &PreferencesUser,
    ) -> Result<()>;
    async fn preferences_user_get(
        &mut self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<PreferencesUser>;
}

#[async_trait]
pub trait DataReaction {
    async fn reaction_put(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_list(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>>;
    async fn reaction_delete(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_delete_key(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()>;
    async fn reaction_delete_all(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()>;
    // TODO: make this return type less terrible
    async fn reaction_fetch_all(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        messages: &[MessageId],
    ) -> Result<Vec<(MessageId, Vec<(ReactionKeyParam, u64, bool)>)>>;
}

#[async_trait]
pub trait DataApplication {
    async fn application_insert(&mut self, data: Application) -> Result<()>;
    async fn application_update(&mut self, data: Application) -> Result<()>;
    async fn application_delete(&mut self, id: ApplicationId) -> Result<()>;
    async fn application_get(&mut self, id: ApplicationId) -> Result<Application>;
    async fn application_list(
        &mut self,
        owner_id: UserId,
        q: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Application>>;
}

#[async_trait]
pub trait DataConnection {
    async fn connection_create(
        &mut self,
        user_id: UserId,
        application_id: ApplicationId,
        scopes: Scopes,
    ) -> Result<()>;
    async fn connection_get(
        &mut self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> Result<Connection>;
    async fn connection_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Connection>>;
    async fn connection_delete(
        &mut self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataEmoji {
    async fn emoji_create(
        &mut self,
        creator_id: UserId,
        room_id: RoomId,
        create: EmojiCustomCreate,
    ) -> Result<EmojiCustom>;
    async fn emoji_get(&mut self, emoji_id: EmojiId) -> Result<EmojiCustom>;
    async fn emoji_get_many(&mut self, emoji_ids: &[EmojiId]) -> Result<Vec<EmojiCustom>>;
    async fn emoji_list(
        &mut self,
        room_id: RoomId,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>>;
    async fn emoji_update(&mut self, emoji_id: EmojiId, patch: EmojiCustomPatch) -> Result<()>;
    async fn emoji_delete(&mut self, emoji_id: EmojiId) -> Result<()>;
    async fn emoji_search(
        &mut self,
        user_id: UserId,
        query: String,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>>;
}

#[async_trait]
pub trait DataUserEmail {
    async fn user_email_add(
        &mut self,
        user_id: UserId,
        email: EmailInfo,
        max_user_emails: usize,
    ) -> Result<()>;
    async fn user_email_delete(&mut self, user_id: UserId, email_addr: EmailAddr) -> Result<()>;
    async fn user_email_list(&mut self, user_id: UserId) -> Result<Vec<EmailInfo>>;
    async fn user_email_lookup(&mut self, email_addr: &EmailAddr) -> Result<UserId>;

    /// check and delete a code, and update is_verified
    async fn user_email_verify_use(
        &mut self,
        user_id: UserId,
        email_addr: EmailAddr,
        code: String,
    ) -> Result<()>;

    /// create a code and update last_updated_at
    async fn user_email_verify_create(
        &mut self,
        user_id: UserId,
        email_addr: EmailAddr,
    ) -> Result<String>;

    async fn user_email_update(
        &mut self,
        user_id: UserId,
        email_addr: EmailAddr,
        patch: EmailInfoPatch,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataDm {
    async fn dm_put(
        &mut self,
        user_a_id: UserId,
        user_b_id: UserId,
        channel_id: ChannelId,
    ) -> Result<()>;
    async fn dm_get(&mut self, user_a_id: UserId, user_b_id: UserId) -> Result<Option<ChannelId>>;
    async fn dm_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Channel>>;
}

#[async_trait]
pub trait DataNotification {
    async fn notification_add(&mut self, user_id: UserId, notif: Notification) -> Result<()>;
    async fn notification_delete(&mut self, user_id: UserId, notif: NotificationId) -> Result<()>;
    async fn notification_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<NotificationId>,
        params: InboxListParams,
    ) -> Result<PaginationResponse<Notification>>;
    async fn notification_mark_read(
        &mut self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()>;
    async fn notification_mark_unread(
        &mut self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()>;
    async fn notification_flush(
        &mut self,
        user_id: UserId,
        params: NotificationFlush,
    ) -> Result<()>;
    async fn notification_get_unpushed(
        &mut self,
        limit: u32,
    ) -> Result<Vec<(UserId, Notification)>>;
    async fn notification_set_pushed(&mut self, ids: &[NotificationId]) -> Result<()>;
}

#[async_trait]
pub trait DataCalendar {
    async fn calendar_event_create(
        &mut self,
        create: CalendarEventCreate,
        channel_id: ChannelId,
        creator_id: UserId,
    ) -> Result<CalendarEvent>;
    async fn calendar_event_get(&mut self, event_id: CalendarEventId) -> Result<CalendarEvent>;
    async fn calendar_event_list(
        &mut self,
        channel_id: ChannelId,
        query: CalendarEventListQuery,
    ) -> Result<PaginationResponse<CalendarEvent>>;
    async fn calendar_event_update(
        &mut self,
        event_id: CalendarEventId,
        patch: CalendarEventPatch,
    ) -> Result<CalendarEvent>;
    async fn calendar_event_delete(&mut self, event_id: CalendarEventId) -> Result<()>;

    // RSVP methods for the event (series)
    async fn calendar_event_rsvp_put(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_event_rsvp_delete(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_event_rsvp_get(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<CalendarEventParticipant>;
    async fn calendar_event_rsvp_list(
        &mut self,
        event_id: CalendarEventId,
        query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>>;

    // Overwrite methods
    async fn calendar_overwrite_put(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        put: CalendarOverwritePut,
    ) -> Result<CalendarOverwrite>;
    async fn calendar_overwrite_get(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
    ) -> Result<CalendarOverwrite>;
    async fn calendar_overwrite_list(
        &mut self,
        event_id: CalendarEventId,
    ) -> Result<Vec<CalendarOverwrite>>;
    async fn calendar_overwrite_delete(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
    ) -> Result<()>;

    // RSVP methods for overwrites
    async fn calendar_overwrite_rsvp_put(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
        attending: bool,
    ) -> Result<()>;
    async fn calendar_overwrite_rsvp_delete(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
    ) -> Result<()>;
    async fn calendar_overwrite_rsvp_list(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>>;
}

#[async_trait]
pub trait DataAdmin {
    /// garbage collect room analytics data
    ///
    /// returns rows affected
    async fn gc_room_analytics(&mut self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// garbage collect messages data
    ///
    /// returns rows affected
    async fn gc_messages(&mut self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// marks media for garbage collection
    async fn gc_media_mark(&mut self) -> Result<u64>;

    /// gets candidates for media garbage collection sweep
    async fn gc_media_get_sweep_candidates(&mut self, limit: u32) -> Result<Vec<MediaId>>;

    /// deletes media that has been swept
    async fn gc_media_delete_swept(&mut self, ids: &[MediaId]) -> Result<u64>;

    /// counts media marked for deletion
    async fn gc_media_count_deleted(&mut self) -> Result<u64>;

    /// garbage collect sessions
    ///
    /// returns rows affected
    async fn gc_sessions(&mut self, mode: AdminCollectGarbageMode) -> Result<u64>;

    /// garbage collect audit logs
    ///
    /// returns rows affected
    async fn gc_audit_logs(&mut self, mode: AdminCollectGarbageMode) -> Result<u64>;
}

#[async_trait]
pub trait DataConfigInternal {
    async fn config_put(&mut self, config: ConfigInternal) -> Result<()>;
    async fn config_get(&mut self) -> Result<Option<ConfigInternal>>;
}
