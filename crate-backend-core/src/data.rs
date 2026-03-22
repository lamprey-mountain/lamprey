use async_trait::async_trait;
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
    RoomMemberSearchAdvanced, RoomMemberSearchResponse, TagId, ThreadMember, ThreadMemberPut,
    UserId, WebhookId,
};
use common::v2::types::message::Message as MessageV2;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{config::ConfigInternal, types::admin::AdminCollectGarbageMode, Result};

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

    /// Check if target user allows DMs from source user (via global or any shared room)
    async fn permission_allows_dm_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool>;

    /// Check if target user allows friend requests from source user (via global or any shared room)
    async fn permission_allows_friend_request_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool>;
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
    async fn search_reindex_queue_get(&self, channel_id: ChannelId) -> Result<Option<MessageId>>;
    async fn search_reindex_queue_upsert_room(&self, room_id: RoomId) -> Result<()>;
    async fn search_reindex_queue_upsert_all(&self) -> Result<()>;
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

    /// check if two users share a room (both are members)
    async fn user_shares_room(&self, user_a: UserId, user_b: UserId) -> Result<bool>;

    /// get shared room IDs between two users (both are members)
    async fn user_shared_rooms(&self, user_a: UserId, user_b: UserId) -> Result<Vec<RoomId>>;

    /// check if two users have at least one mutual friend
    async fn user_has_mutual_friend(&self, user_a: UserId, user_b: UserId) -> Result<bool>;
}

#[async_trait]
pub trait DataPreferences {
    async fn preferences_set(&self, user_id: UserId, config: &PreferencesGlobal) -> Result<()>;
    async fn preferences_get(&self, user_id: UserId) -> Result<PreferencesGlobal>;
    async fn preferences_room_set(
        &self,
        user_id: UserId,
        room_id: RoomId,
        config: &PreferencesRoom,
    ) -> Result<()>;
    async fn preferences_room_get(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<PreferencesRoom>;
    async fn preferences_room_get_many(
        &self,
        user_id: UserId,
        room_ids: &[RoomId],
    ) -> Result<HashMap<RoomId, PreferencesRoom>>;
    async fn preferences_channel_set(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        config: &PreferencesChannel,
    ) -> Result<()>;
    async fn preferences_channel_get(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<PreferencesChannel>;
    async fn preferences_channel_get_many(
        &self,
        user_id: UserId,
        channel_ids: &[ChannelId],
    ) -> Result<HashMap<ChannelId, PreferencesChannel>>;
    async fn preferences_user_set(
        &self,
        user_id: UserId,
        other_id: UserId,
        config: &PreferencesUser,
    ) -> Result<()>;
    async fn preferences_user_get(
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
    async fn emoji_get_many(&self, emoji_ids: &[EmojiId]) -> Result<Vec<EmojiCustom>>;
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
pub trait DataConfigInternal {
    async fn config_put(&self, config: ConfigInternal) -> Result<()>;
    async fn config_get(&self) -> Result<Option<ConfigInternal>>;
}
