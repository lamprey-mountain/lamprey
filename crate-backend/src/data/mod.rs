use async_trait::async_trait;
use common::v1::types::application::{Application, Connection, Scope};
use common::v1::types::email::{EmailAddr, EmailInfo, EmailInfoPatch};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch};
use common::v1::types::notifications::{
    InboxListParams, InboxThreadsParams, Notification, NotificationFlush, NotificationMarkRead,
};
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::search::{SearchMessageRequest, SearchThreadsRequest};
use common::v1::types::user_config::UserConfigGlobal;
use common::v1::types::util::Time;
use common::v1::types::{
    ApplicationId, AuditLogEntry, AuditLogEntryId, Embed, EmojiId, InvitePatch, InviteWithMetadata,
    MediaPatch, NotificationId, Permission, PermissionOverwriteType, PinsReorder, Relationship,
    RelationshipPatch, RelationshipWithUserId, Role, RoleReorder, RoomBan, RoomMember,
    RoomMemberOrigin, RoomMemberPatch, RoomMemberPut, RoomMembership, RoomMetrics, SessionPatch,
    SessionStatus, SessionToken, Suspended, ThreadMember, ThreadMemberPut, ThreadMembership,
    ThreadReorder, UserListFilter,
};

use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbEmailQueue, DbMessageCreate, DbRoleCreate, DbRoomCreate, DbSessionCreate, DbThreadCreate,
    DbThreadPrivate, DbUserCreate, EmailPurpose, InviteCode, Media, MediaId, MediaLink,
    MediaLinkType, Message, MessageId, MessageRef, MessageVerId, PaginationQuery,
    PaginationResponse, Permissions, RoleId, RolePatch, RoleVerId, Room, RoomCreate, RoomId,
    RoomPatch, RoomVerId, Session, SessionId, Thread, ThreadId, ThreadPatch, ThreadVerId,
    UrlEmbedQueue, User, UserId, UserPatch, UserVerId,
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
    + DataThread
    + DataUnread
    + DataUser
    + DataSearch
    + DataAuth
    + DataAuditLogs
    + DataThreadMember
    + DataUserRelationship
    + DataUserConfig
    + DataReaction
    + DataApplication
    + DataConnection
    + DataEmoji
    + DataEmbed
    + DataUserEmail
    + DataEmailQueue
    + DataDm
    + DataNotification
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
    async fn room_metrics(&self, room_id: RoomId) -> Result<RoomMetrics>;
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
pub trait DataRole {
    async fn role_create(&self, create: DbRoleCreate, position: u64) -> Result<Role>;
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
    async fn role_member_put(&self, user_id: UserId, role_id: RoleId) -> Result<()>;
    async fn role_member_delete(&self, user_id: UserId, role_id: RoleId) -> Result<()>;
    async fn role_member_list(
        &self,
        role_id: RoleId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>>;
    async fn role_member_count(&self, role_id: RoleId) -> Result<u64>;
    async fn role_member_bulk_edit(
        &self,
        role_id: RoleId,
        apply_user_ids: &[UserId],
        remove_user_ids: &[UserId],
    ) -> Result<()>;
}

#[async_trait]
pub trait DataPermission {
    async fn permission_room_get(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions>;
    async fn permission_thread_get(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
    ) -> Result<Permissions>;
    async fn permission_is_mutual(&self, a: UserId, b: UserId) -> Result<bool>;
    async fn permission_overwrite_upsert(
        &self,
        thread_id: ThreadId,
        overwrite_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    ) -> Result<()>;

    async fn permission_overwrite_delete(
        &self,
        thread_id: ThreadId,
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
    async fn media_select(&self, media_id: MediaId) -> Result<(Media, UserId)>;
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
        thread_id: ThreadId,
        message_id: MessageId,
        create: DbMessageCreate,
    ) -> Result<MessageVerId>;
    async fn message_update_in_place(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
        create: DbMessageCreate,
    ) -> Result<()>;
    async fn message_get(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
    ) -> Result<Message>;
    async fn message_list(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()>;
    async fn message_delete_bulk(
        &self,
        thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_remove_bulk(
        &self,
        thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_restore_bulk(
        &self,
        thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
        user_id: UserId,
    ) -> Result<Message>;
    async fn message_version_delete(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<()>;
    async fn message_version_list(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_replies(
        &self,
        thread_id: ThreadId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_pin_create(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()>;
    async fn message_pin_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()>;
    async fn message_pin_reorder(&self, thread_id: ThreadId, reorder: PinsReorder) -> Result<()>;
    async fn message_pin_list(
        &self,
        thread_id: ThreadId,
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
    async fn session_set_last_seen_at(&self, session_id: SessionId) -> Result<()>;
}

#[async_trait]
pub trait DataThread {
    async fn thread_create(&self, create: DbThreadCreate) -> Result<ThreadId>;
    async fn thread_get(&self, thread_id: ThreadId) -> Result<Thread>;
    async fn thread_get_private(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
    ) -> Result<DbThreadPrivate>;
    async fn thread_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
        parent_id: Option<ThreadId>,
    ) -> Result<PaginationResponse<Thread>>;
    async fn thread_list_archived(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
        parent_id: Option<ThreadId>,
    ) -> Result<PaginationResponse<Thread>>;
    async fn thread_list_removed(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
        parent_id: Option<ThreadId>,
    ) -> Result<PaginationResponse<Thread>>;
    async fn thread_update(&self, thread_id: ThreadId, patch: ThreadPatch) -> Result<ThreadVerId>;
    async fn thread_delete(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_archive(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_unarchive(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_undelete(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_lock(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_unlock(&self, thread_id: ThreadId) -> Result<()>;
    async fn thread_reorder(&self, data: ThreadReorder) -> Result<()>;
    async fn thread_upgrade_gdm(&self, thread_id: ThreadId, room_id: RoomId) -> Result<()>;
}

#[async_trait]
pub trait DataUnread {
    async fn unread_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataUser {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User>;
    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId>;
    async fn user_delete(&self, user_id: UserId) -> Result<()>;
    async fn user_undelete(&self, user_id: UserId) -> Result<()>;
    async fn user_get(&self, user_id: UserId) -> Result<User>;
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
    ) -> Result<PaginationResponse<Message>>;
    async fn search_thread(
        &self,
        user_id: UserId,
        query: SearchThreadsRequest,
        paginate: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>>;
}

#[async_trait]
pub trait DataAuditLogs {
    async fn audit_logs_room_fetch(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogEntryId>,
    ) -> Result<PaginationResponse<AuditLogEntry>>;
    async fn audit_logs_room_append(&self, entry: AuditLogEntry) -> Result<()>;
}

#[async_trait]
pub trait DataThreadMember {
    /// is a no-op if membership won't change
    async fn thread_member_put(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        put: ThreadMemberPut,
    ) -> Result<()>;
    async fn thread_member_set_membership(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()>;
    async fn thread_member_delete(&self, thread_id: ThreadId, user_id: UserId) -> Result<()>;
    async fn thread_member_get(&self, thread_id: ThreadId, user_id: UserId)
        -> Result<ThreadMember>;
    async fn thread_member_list(
        &self,
        thread_id: ThreadId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>>;

    async fn thread_member_list_all(&self, thread_id: ThreadId) -> Result<Vec<ThreadMember>>;
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
    async fn user_relationship_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>>;
}

#[async_trait]
pub trait DataUserConfig {
    async fn user_config_set(&self, user_id: UserId, config: &UserConfigGlobal) -> Result<()>;
    async fn user_config_get(&self, user_id: UserId) -> Result<UserConfigGlobal>;
}

#[async_trait]
pub trait DataReaction {
    async fn reaction_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_delete(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_list(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>>;
    async fn reaction_purge(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()>;
    async fn reaction_purge_key(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()>;
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
    async fn dm_put(&self, user_a_id: UserId, user_b_id: UserId, thread_id: ThreadId)
        -> Result<()>;
    async fn dm_get(&self, user_a_id: UserId, user_b_id: UserId) -> Result<Option<ThreadId>>;
    async fn dm_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Thread>>;
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
    async fn notification_list_threads(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ThreadId>,
        params: InboxThreadsParams,
        list_params: InboxListParams,
    ) -> Result<PaginationResponse<Thread>>;
}
