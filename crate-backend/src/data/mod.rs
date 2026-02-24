// TODO: rename foo_select to foo_get

use async_trait::async_trait;
use common::v1::types::calendar::{Calendar, CalendarPatch};
use common::v1::types::document::{
    Document, DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchPatch,
    DocumentBranchState, DocumentPatch, DocumentTag, Wiki, WikiPatch,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::media::MediaWithAdmin;
use common::v1::types::oauth::Scopes;
use common::v1::types::util::Time;

use common::v1::types::{
    ApplicationId, Channel, ChannelId, ChannelPatch, ChannelReorder, ChannelVerId,
    DocumentBranchId, DocumentTagId, Embed, Media, MediaId, MediaPatch, PaginationQuery,
    PaginationResponse, PinsReorder, Role, RoleId, RolePatch, RoleReorder, RoleVerId, Room,
    RoomCreate, RoomId, RoomPatch, RoomVerId, Session, SessionId, SessionPatch, SessionStatus,
    SessionToken, Suspended, User, UserId, UserListFilter,
};

use common::v2::types::media::Media as MediaV2;
use common::v2::types::message::{Message as MessageV2, MessageVersion as MessageVersionV2};

use lamprey_backend_core::data::{
    DataAdmin, DataApplication, DataAuditLogs, DataAutomod, DataCalendar, DataConfigInternal,
    DataConnection, DataDm, DataEmoji, DataInvite, DataMetrics, DataNotification, DataPermission,
    DataPreferences, DataReaction, DataRoleMember, DataRoomAnalytics, DataRoomMember, DataSearch,
    DataSearchQueue, DataTag, DataThread, DataThreadMember, DataUnread, DataUserEmail,
    DataUserRelationship, DataWebhook,
};
use uuid::Uuid;

use crate::error::Result;
use crate::services::documents::EditContextId;
use crate::types::{
    DbChannelCreate, DbChannelPrivate, DbEmailQueue, DbMessageCreate, DbRoleCreate, DbRoomCreate,
    DbSessionCreate, DbUserCreate, DehydratedDocument, DocumentUpdateSummary, EmailPurpose,
    MediaLink, MediaLinkType, MentionsIds, MessageId, MessageRef, MessageVerId, PushData,
    UrlEmbedQueue, UserPatch, UserVerId,
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
    + DataMediaV1
    + DataMediaV2
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
    + Send
    + Sync
{
    // async fn commit(self) -> Result<()>;
    // async fn rollback(self) -> Result<()>;
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
    async fn user_owns_room_requiring_mfa(&self, user_id: UserId) -> Result<bool>;
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
pub trait DataMediaV1 {
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
pub trait DataMediaV2 {
    async fn media2_insert(&self, media: MediaV2) -> Result<()>;
    async fn media2_select(&self, media_id: MediaId) -> Result<MediaV2>;
    async fn media2_update(
        &self,
        media_id: MediaId,
        patch: common::v2::types::media::MediaPatch,
    ) -> Result<()>;
    async fn media2_delete(&self, media_id: MediaId) -> Result<()>;
    async fn media2_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;
    async fn media2_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>>;
    async fn media2_link_delete(&self, target_id: Uuid, link_type: MediaLinkType) -> Result<()>;
    async fn media2_link_delete_all(&self, target_id: Uuid) -> Result<()>;
    async fn media2_link_create_exclusive(
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
    async fn message_pin_create(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<bool>;
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
    async fn channel_ratelimit_delete_all(&self, channel_id: ChannelId) -> Result<()>;

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
