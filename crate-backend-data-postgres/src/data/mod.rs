pub mod postgres;
use std::collections::HashMap;

use crate::EditContextId;

use async_trait::async_trait;
use common::v1::types::calendar::{Calendar, CalendarPatch};
use common::v1::types::document::{
    Document, DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchPatch,
    DocumentBranchState, DocumentPatch, DocumentTag, Wiki, WikiPatch,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::federation::{Hostname, Remote};
use common::v1::types::message::{Message, MessageVersion};
use common::v1::types::oauth::Scopes;
use common::v1::types::room_template::{RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch};
use common::v1::types::sync::ChannelSync;
use common::v1::types::util::Time;
use common::v1::types::{
    ApplicationId, Channel, ChannelId, ChannelPatch, ChannelReorder, ChannelVerId,
    DocumentBranchId, DocumentTagId, MediaId, MediaVerId, PaginationQuery, PaginationResponse,
    PinsReorder, Role, RoleId, RolePatch, RoleReorder, RoleVerId, Room, RoomCreate, RoomId,
    RoomPatch, RoomVerId, Session, SessionId, SessionImprint, SessionPatch, SessionStatus,
    SessionToken, Suspended, User, UserId, UserListFilter,
};
use common::v1::types::{ChannelSeq, RoomFeature};
use common::v2::types::embed::Embed;
use common::v2::types::media::{Media, MediaPatch};
use lamprey_backend_core::data::DataScript;
pub use lamprey_backend_core::data::{
    DataAdmin, DataApplication, DataAuditLogs, DataAutomod, DataCalendar, DataConfigInternal,
    DataConnection, DataDm, DataEmoji, DataInvite, DataMetrics, DataNotification, DataPermission,
    DataPreferences, DataReaction, DataRoleMember, DataRoomAnalytics, DataRoomMember,
    DataSearchQueue, DataTag, DataThread, DataThreadMember, DataUnread, DataUserEmail,
    DataUserRelationship, DataWebhook,
};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbChannelCreate, DbChannelPrivate, DbEmailQueue, DbMessageCreate, DbMessageUpdate,
    DbRoleCreate, DbRoomCreate, DbRoomTemplate, DbSessionCreate, DbUserCreate, DehydratedDocument,
    DocumentUpdateSummary, EmailPurpose, MediaLink, MediaLinkType, MentionsIds, MessageId,
    MessageRef, MessageVerId, PushData, UrlEmbedQueue, UserPatch, UserVerId,
};
use common::v1::types::components::{self, Components};

#[async_trait]
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
    + DataSearchQueue
    + DataAuth
    + DataAuditLogs
    + DataThreadMember
    + DataThread
    + DataUserRelationship
    + DataPreferences
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
    + DataRoomTemplate
    + DataScript
    + Send
    + Sync
{
    async fn rollback(self: Box<Self>) -> Result<()>;
    async fn commit(self: Box<Self>) -> Result<()>;
}

#[async_trait]
pub trait Data2: Send + Sync {
    // TODO: find a way to erase this type
    type DataTxn: Data;

    async fn migrate(&self) -> Result<()>;
    async fn check_database(&self) -> Result<bool>;
    async fn begin(&self) -> Result<Self::DataTxn>;
}

#[async_trait]
pub trait DataRoom {
    async fn room_create(&mut self, create: RoomCreate, extra: DbRoomCreate) -> Result<Room>;
    async fn room_get(&mut self, room_id: RoomId) -> Result<Room>;
    async fn room_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<RoomId>,
        include_server_room: bool,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_list_all(
        &mut self,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_list_user_all(&mut self, user_id: UserId) -> Result<Vec<RoomId>>;
    async fn room_list_mutual(
        &mut self,
        user_a_id: UserId,
        user_b_id: UserId,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_update(&mut self, room_id: RoomId, patch: RoomPatch) -> Result<RoomVerId>;
    async fn room_set_owner(&mut self, id: RoomId, owner_id: UserId) -> Result<RoomVerId>;
    async fn room_delete(&mut self, room_id: RoomId) -> Result<()>;
    async fn room_undelete(&mut self, room_id: RoomId) -> Result<()>;
    async fn room_quarantine(&mut self, room_id: RoomId) -> Result<RoomVerId>;
    async fn room_unquarantine(&mut self, room_id: RoomId) -> Result<RoomVerId>;
    async fn user_room_count(&mut self, user_id: UserId) -> Result<u64>;
    async fn room_security_update(
        &mut self,
        room_id: RoomId,
        require_mfa: Option<bool>,
        require_sudo: Option<bool>,
    ) -> Result<RoomVerId>;
    async fn user_owns_room_requiring_mfa(&mut self, user_id: UserId) -> Result<bool>;
    async fn room_set_features(
        &mut self,
        room_id: RoomId,
        features: &[RoomFeature],
    ) -> Result<RoomVerId>;
}

#[async_trait]
pub trait DataRole {
    async fn role_create(&mut self, create: DbRoleCreate, position: u64) -> Result<Role>;
    async fn role_list(&mut self, room_id: RoomId) -> Result<Vec<Role>>;
    async fn role_delete(&mut self, room_id: RoomId, role_id: RoleId) -> Result<()>;
    async fn role_select(&mut self, room_id: RoomId, role_id: RoleId) -> Result<Role>;
    async fn role_get_many(&mut self, room_id: RoomId, role_ids: &[RoleId]) -> Result<Vec<Role>>;
    async fn role_update(
        &mut self,
        room_id: RoomId,
        role_id: RoleId,
        patch: RolePatch,
    ) -> Result<RoleVerId>;
    async fn role_reorder(&mut self, room_id: RoomId, reorder: RoleReorder) -> Result<()>;
    async fn role_user_rank(&mut self, room_id: RoomId, user_id: UserId) -> Result<u64>;
}

#[async_trait]
pub trait DataMedia {
    async fn media_insert(&mut self, media: Media) -> Result<()>;
    async fn media_select(&mut self, media_id: MediaId) -> Result<Media>;
    async fn media_update(&mut self, media_id: MediaId, patch: MediaPatch) -> Result<()>;
    async fn media_replace(&mut self, media: Media) -> Result<()>;
    async fn media_delete(&mut self, media_id: MediaId) -> Result<()>;
    async fn media_link_insert(
        &mut self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;
    async fn media_link_select(&mut self, media_id: MediaId) -> Result<Vec<MediaLink>>;
    async fn media_link_delete(&mut self, target_id: Uuid, link_type: MediaLinkType) -> Result<()>;
    async fn media_link_delete_all(&mut self, target_id: Uuid) -> Result<()>;
    async fn media_link_create_exclusive(
        &mut self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;
    async fn media_migrate_batch(&mut self, limit: u32) -> Result<u64>;
    async fn media_list_indexed(
        &mut self,
        after_version_id: Option<MediaVerId>,
        limit: u32,
    ) -> Result<Vec<Media>>;
    async fn media_select_by_remote(
        &mut self,
        hostname: &Hostname,
        origin_id: Uuid,
    ) -> Result<Option<Media>>;
}

#[async_trait]
pub trait DataMessage {
    async fn message_create(&mut self, create: DbMessageCreate) -> Result<MessageId>;
    async fn message_update(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        update: DbMessageUpdate,
    ) -> Result<MessageVerId>;
    async fn message_update_in_place(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
        update: DbMessageUpdate,
    ) -> Result<()>;
    async fn message_flume_update(
        &mut self,
        message_id: MessageId,
        flume: serde_json::Value,
    ) -> Result<()>;
    async fn message_get(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message>;
    async fn message_get_many(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<Vec<Message>>;
    async fn message_list(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_deleted(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_removed(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_activity(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_list_all(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_delete(&mut self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
    async fn message_delete_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_remove_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_restore_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_version_get(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageVersion>;
    async fn message_version_delete(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<()>;
    async fn message_version_list(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<MessageVersion>>;
    async fn message_replies(
        &mut self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_pin_create(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<bool>;
    async fn message_pin_delete(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()>;
    async fn message_pin_reorder(
        &mut self,
        channel_id: ChannelId,
        reorder: PinsReorder,
    ) -> Result<()>;
    async fn message_pin_list(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_get_ancestors(
        &mut self,
        message_id: MessageId,
        limit: u16,
    ) -> Result<Vec<Message>>;
    async fn message_fetch_mention_ids(
        &mut self,
        channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<Vec<MentionsIds>>;
    async fn message_fetch_components(
        &mut self,
        channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<HashMap<MessageVerId, Components<components::Thin>>>;
    async fn message_id_get_by_version(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageId>;

    /// Get incremental sync events for a channel since the given sequence number.
    async fn channel_sync(
        &mut self,
        channel_id: ChannelId,
        since: ChannelSeq,
        pagination: PaginationQuery<MessageId>,
        user_id: Option<UserId>,
    ) -> Result<ChannelSync>;
}

#[async_trait]
pub trait DataSession {
    async fn session_create(&mut self, create: DbSessionCreate) -> Result<Session>;
    async fn session_get(&mut self, session_id: SessionId) -> Result<Session>;
    async fn session_get_by_token(&mut self, token: SessionToken) -> Result<Session>;
    async fn session_set_status(
        &mut self,
        session_id: SessionId,
        status: SessionStatus,
    ) -> Result<()>;
    async fn session_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>>;
    async fn session_update(&mut self, session_id: SessionId, patch: SessionPatch) -> Result<()>;
    async fn session_delete(&mut self, session_id: SessionId) -> Result<()>;
    async fn session_delete_all(&mut self, user_id: UserId) -> Result<()>;
    async fn session_set_last_seen_at(&mut self, session_id: SessionId) -> Result<()>; // TODO: remove
    async fn session_update_imprint(
        &mut self,
        session_id: SessionId,
        imprint: SessionImprint,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataChannel {
    async fn channel_create(&mut self, create: DbChannelCreate) -> Result<ChannelId>;
    async fn channel_create_with_id(
        &mut self,
        id: ChannelId,
        create: DbChannelCreate,
    ) -> Result<()>;
    async fn channel_get(&mut self, channel_id: ChannelId) -> Result<Channel>;
    async fn channel_get_many(&mut self, channel_ids: &[ChannelId]) -> Result<Vec<Channel>>;
    async fn channel_get_private(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<DbChannelPrivate>;
    // async fn channel_list_all(&mut self, p: PaginationQuery<ChannelId>) -> Result<PaginationResult<Channel>>;
    async fn channel_list(&mut self, room_id: RoomId) -> Result<Vec<Channel>>;
    async fn channel_list_all(
        &mut self,
        p: PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>>;
    async fn channel_list_removed(
        &mut self,
        room_id: RoomId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>>;
    async fn channel_update(
        &mut self,
        channel_id: ChannelId,
        patch: ChannelPatch,
    ) -> Result<ChannelVerId>;
    async fn channel_delete(&mut self, channel_id: ChannelId) -> Result<()>;
    async fn channel_undelete(&mut self, channel_id: ChannelId) -> Result<()>;
    async fn channel_reorder(&mut self, data: ChannelReorder) -> Result<()>;
    async fn channel_upgrade_gdm(&mut self, channel_id: ChannelId, room_id: RoomId) -> Result<()>;
    async fn channel_get_message_slowmode_expire_at(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>>;
    async fn channel_set_message_slowmode_expire_at(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()>;
    async fn channel_get_thread_slowmode_expire_at(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>>;
    async fn channel_set_thread_slowmode_expire_at(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()>;
    async fn channel_ratelimit_delete_all(&mut self, channel_id: ChannelId) -> Result<()>;
    async fn channel_document_insert(
        &mut self,
        channel_id: ChannelId,
        document: &Document,
    ) -> Result<()>;
    async fn channel_document_get(&mut self, channel_id: ChannelId) -> Result<Option<Document>>;
    async fn channel_document_update(
        &mut self,
        channel_id: ChannelId,
        document_patch: &DocumentPatch,
    ) -> Result<()>;
    async fn channel_wiki_insert(&mut self, channel_id: ChannelId, wiki: &Wiki) -> Result<()>;
    async fn channel_wiki_get(&mut self, channel_id: ChannelId) -> Result<Option<Wiki>>;
    async fn channel_wiki_update(
        &mut self,
        channel_id: ChannelId,
        wiki_patch: &WikiPatch,
    ) -> Result<()>;
    async fn channel_calendar_insert(
        &mut self,
        channel_id: ChannelId,
        calendar: &Calendar,
    ) -> Result<()>;
    async fn channel_calendar_get(&mut self, channel_id: ChannelId) -> Result<Option<Calendar>>;
    async fn channel_calendar_update(
        &mut self,
        channel_id: ChannelId,
        calendar_patch: &CalendarPatch,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataUser {
    async fn user_create(&mut self, patch: DbUserCreate) -> Result<User>;
    async fn user_update(&mut self, user_id: UserId, patch: UserPatch) -> Result<UserVerId>;
    async fn user_delete(&mut self, user_id: UserId) -> Result<()>;
    async fn user_undelete(&mut self, user_id: UserId) -> Result<()>;
    async fn user_get(&mut self, user_id: UserId) -> Result<User>;
    async fn user_get_remote(&mut self, remote: &Remote) -> Result<User>;
    async fn user_get_many(&mut self, user_ids: &[UserId]) -> Result<Vec<User>>;
    async fn user_list(
        &mut self,
        pagination: PaginationQuery<UserId>,
        filter: Option<UserListFilter>,
    ) -> Result<PaginationResponse<User>>;
    async fn user_lookup_puppet(
        &mut self,
        owner_id: UserId,
        external_id: &str,
    ) -> Result<Option<UserId>>;
    async fn user_set_registered(
        &mut self,
        user_id: UserId,
        registered_at: Option<Time>,
        parent_invite: Option<String>,
    ) -> Result<UserVerId>;
    async fn user_suspended(
        &mut self,
        user_id: UserId,
        suspended: Option<Suspended>,
    ) -> Result<UserVerId>;
}

#[async_trait]
pub trait DataAuth {
    async fn auth_oauth_put(
        &mut self,
        provider: String,
        user_id: UserId,
        remote_id: String,
        can_auth: bool,
    ) -> Result<()>;
    async fn auth_oauth_get_all(&mut self, user_id: UserId) -> Result<Vec<String>>;
    async fn auth_oauth_get_remote(
        &mut self,
        provider: String,
        remote_id: String,
    ) -> Result<Option<UserId>>;
    async fn auth_oauth_delete(&mut self, provider: String, user_id: UserId) -> Result<()>;
    async fn auth_password_set(&mut self, user_id: UserId, hash: &[u8], salt: &[u8]) -> Result<()>;
    async fn auth_password_get(&mut self, user_id: UserId) -> Result<Option<(Vec<u8>, Vec<u8>)>>;
    async fn auth_password_delete(&mut self, user_id: UserId) -> Result<()>;
    async fn auth_email_create(
        &mut self,
        code: String,
        addr: EmailAddr,
        session_id: SessionId,
        purpose: EmailPurpose,
    ) -> Result<()>;
    async fn auth_email_use(
        &mut self,
        code: String,
    ) -> Result<(EmailAddr, SessionId, EmailPurpose)>;
    async fn auth_totp_set(
        &mut self,
        user_id: UserId,
        secret: Option<String>,
        enabled: bool,
    ) -> Result<()>;
    async fn auth_totp_get(&mut self, user_id: UserId) -> Result<Option<(String, bool)>>;
    async fn auth_totp_recovery_generate(
        &mut self,
        user_id: UserId,
        codes: &[String],
    ) -> Result<()>;
    async fn auth_totp_recovery_get_all(
        &mut self,
        user_id: UserId,
    ) -> Result<Vec<(String, Option<Time>)>>;
    async fn auth_totp_recovery_use(&mut self, user_id: UserId, code: &str) -> Result<()>;
    async fn auth_totp_recovery_delete_all(&mut self, user_id: UserId) -> Result<()>;
    async fn oauth_auth_code_create(
        &mut self,
        code: String,
        application_id: ApplicationId,
        user_id: UserId,
        redirect_uri: String,
        scopes: Scopes,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> Result<()>;
    async fn oauth_auth_code_use(
        &mut self,
        code: String,
    ) -> Result<(
        ApplicationId,
        UserId,
        String,
        Scopes,
        Option<String>,
        Option<String>,
    )>;
    async fn oauth_refresh_token_create(
        &mut self,
        token: String,
        session_id: SessionId,
    ) -> Result<()>;
    async fn oauth_refresh_token_use(&mut self, token: String) -> Result<SessionId>;
}

#[async_trait]
pub trait DataEmbed {
    async fn url_embed_queue_insert(
        &mut self,
        message_ref: Option<MessageRef>,
        user_id: Option<UserId>,
        url: String,
    ) -> Result<Uuid>;
    async fn url_embed_queue_claim(&mut self) -> Result<Option<UrlEmbedQueue>>;
    async fn url_embed_queue_finish(&mut self, id: Uuid, embed: Option<&Embed>) -> Result<()>;
}

#[async_trait]
pub trait DataEmailQueue {
    async fn email_queue_insert(
        &mut self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid>;
    async fn email_queue_claim(&mut self) -> Result<Option<DbEmailQueue>>;
    async fn email_queue_finish(&mut self, id: Uuid) -> Result<()>;
    async fn email_queue_fail(&mut self, error_message: String, id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait DataDocument {
    async fn document_compact(
        &mut self,
        context_id: EditContextId,
        last_snapshot_id: Uuid,
        last_seq: u32,
        snapshot: Vec<u8>,
    ) -> Result<()>;
    async fn document_load(&mut self, context_id: EditContextId) -> Result<DehydratedDocument>;
    async fn document_load_at_seq(
        &mut self,
        context_id: EditContextId,
        seq: u32,
    ) -> Result<DehydratedDocument>;
    async fn document_create(
        &mut self,
        context_id: EditContextId,
        creator_id: UserId,
        snapshot: Vec<u8>,
    ) -> Result<()>;
    async fn document_update(
        &mut self,
        context_id: EditContextId,
        author_id: UserId,
        update: Vec<u8>,
        stat_added: u32,
        stat_removed: u32,
    ) -> Result<u32>;
    async fn document_fork(
        &mut self,
        context_id: EditContextId,
        creator_id: UserId,
        create: DocumentBranchCreate,
    ) -> Result<DocumentBranchId>;
    async fn document_branch_get(
        &mut self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
    ) -> Result<DocumentBranch>;
    async fn document_branch_update(
        &mut self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
        patch: DocumentBranchPatch,
    ) -> Result<()>;
    async fn document_branch_set_state(
        &mut self,
        document_id: ChannelId,
        branch_id: DocumentBranchId,
        status: DocumentBranchState,
    ) -> Result<()>;
    async fn document_branch_list(&mut self, document_id: ChannelId)
        -> Result<Vec<DocumentBranch>>;
    async fn document_branch_paginate(
        &mut self,
        document_id: ChannelId,
        user_id: UserId,
        filter: DocumentBranchListParams,
        pagination: PaginationQuery<DocumentBranchId>,
    ) -> Result<PaginationResponse<DocumentBranch>>;
    async fn document_tag_create(
        &mut self,
        branch_id: DocumentBranchId,
        creator_id: UserId,
        summary: String,
        description: Option<String>,
        revision_seq: u64,
    ) -> Result<DocumentTagId>;
    async fn document_tag_get(&mut self, tag_id: DocumentTagId) -> Result<DocumentTag>;
    async fn document_tag_update(
        &mut self,
        tag_id: DocumentTagId,
        summary: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<()>;
    async fn document_tag_delete(&mut self, tag_id: DocumentTagId) -> Result<()>;
    async fn document_tag_list(&mut self, branch_id: DocumentBranchId) -> Result<Vec<DocumentTag>>;
    async fn document_tag_list_by_document(
        &mut self,
        document_id: ChannelId,
        user_id: UserId,
    ) -> Result<Vec<DocumentTag>>;
    async fn document_history(
        &mut self,
        context_id: EditContextId,
    ) -> Result<(Vec<DocumentUpdateSummary>, Vec<DocumentTag>)>;
    async fn wiki_history(
        &mut self,
        wiki_id: ChannelId,
    ) -> Result<(Vec<DocumentUpdateSummary>, Vec<DocumentTag>)>;
}

#[async_trait]
pub trait DataPush {
    async fn push_insert(&mut self, push: PushData) -> Result<()>;
    async fn push_get(&mut self, session_id: SessionId) -> Result<PushData>;
    async fn push_delete(&mut self, session_id: SessionId) -> Result<()>;
    async fn push_list_for_user(&mut self, user_id: UserId) -> Result<Vec<PushData>>;
    async fn push_delete_for_user(&mut self, user_id: UserId) -> Result<()>;
}

#[async_trait]
pub trait DataRoomTemplate {
    async fn room_template_create(
        &mut self,
        creator_id: UserId,
        snapshot: serde_json::Value,
        create: RoomTemplateCreate,
    ) -> Result<DbRoomTemplate>;
    async fn room_template_get(&mut self, code: RoomTemplateCode) -> Result<DbRoomTemplate>;
    async fn room_template_list(
        &mut self,
        creator_id: UserId,
        pagination: PaginationQuery<RoomTemplateCode>,
    ) -> Result<PaginationResponse<DbRoomTemplate>>;
    async fn room_template_update(
        &mut self,
        code: RoomTemplateCode,
        patch: RoomTemplatePatch,
    ) -> Result<DbRoomTemplate>;
    async fn room_template_update_snapshot(
        &mut self,
        code: RoomTemplateCode,
        snapshot: serde_json::Value,
    ) -> Result<DbRoomTemplate>;
    async fn room_template_mark_dirty(&mut self, source_room_id: RoomId) -> Result<()>;
    async fn room_template_delete(&mut self, code: RoomTemplateCode) -> Result<()>;
}
