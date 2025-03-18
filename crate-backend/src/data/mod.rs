use std::time::Duration;

use async_trait::async_trait;
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::search::SearchMessageRequest;
use common::v1::types::user_config::UserConfig;
use common::v1::types::{
    AuditLog, AuditLogId, InviteWithMetadata, MediaPatch, MessageSync, Relationship,
    RelationshipPatch, RelationshipWithUserId, Role, RoomMember, RoomMemberPatch, RoomMembership,
    SessionPatch, SessionStatus, SessionToken, ThreadMember, ThreadMemberPatch, ThreadMembership,
    UrlEmbed, UrlEmbedId,
};
use url::Url;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbMessageCreate, DbRoleCreate, DbThreadCreate, DbUserCreate, InviteCode, Media, MediaId,
    MediaLink, MediaLinkType, Message, MessageId, MessageVerId, PaginationQuery,
    PaginationResponse, Permissions, RoleId, RolePatch, RoleVerId, Room, RoomCreate, RoomId,
    RoomPatch, RoomVerId, Session, SessionId, Thread, ThreadId, ThreadPatch, ThreadVerId, User,
    UserId, UserPatch, UserVerId,
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
    + DataUrlEmbed
    + DataDm
    + DataUserRelationship
    + DataUserConfig
    + DataReaction
    + Send
    + Sync
{
    // async fn commit(self) -> Result<()>;
    // async fn rollback(self) -> Result<()>;
}

#[async_trait]
pub trait DataRoom {
    async fn room_create(&self, create: RoomCreate) -> Result<Room>;
    async fn room_get(&self, room_id: RoomId) -> Result<Room>;
    async fn room_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>>;
    async fn room_update(&self, room_id: RoomId, patch: RoomPatch) -> Result<RoomVerId>;
}

#[async_trait]
pub trait DataRoomMember {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
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
}

#[async_trait]
pub trait DataRole {
    async fn role_create(&self, create: DbRoleCreate) -> Result<Role>;
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
    async fn role_apply_default(&self, room_id: RoomId, user_id: UserId) -> Result<()>;
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
    ) -> Result<()>;
    async fn invite_list_room(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>>;

    // TODO: user invites
    // async fn invite_insert_user(
    //     &self,
    //     user_id: UserId,
    //     creator_id: UserId,
    //     code: InviteCode,
    // ) -> Result<InviteWithMetadata>;
    // async fn invite_list_user(user_id: UserId, paginate: PaginationQuery<InviteCode>) -> Result<PaginationResponse<Invite>>;

    async fn invite_incr_use(&self, target_id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait DataMedia {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<()>;

    async fn media_select(&self, media_id: MediaId) -> Result<(Media, UserId)>;

    async fn media_update(&self, media_id: MediaId, patch: MediaPatch) -> Result<()>;

    async fn media_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()>;

    async fn media_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>>;

    async fn media_link_delete(&self, target_id: Uuid, link_type: MediaLinkType) -> Result<()>;

    async fn media_link_delete_all(&self, target_id: Uuid) -> Result<()>;
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
    async fn message_get(&self, thread_id: ThreadId, message_id: MessageId) -> Result<Message>;
    async fn message_list(
        &self,
        thread_id: ThreadId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
    async fn message_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()>;
    async fn message_delete_bulk(
        &self,
        thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()>;
    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
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
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>>;
}

#[async_trait]
pub trait DataSession {
    async fn session_create(&self, token: SessionToken, name: Option<String>) -> Result<Session>;
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
}

#[async_trait]
pub trait DataThread {
    async fn thread_create(&self, create: DbThreadCreate) -> Result<ThreadId>;
    async fn thread_get(&self, thread_id: ThreadId, user_id: Option<UserId>) -> Result<Thread>;
    async fn thread_list(
        &self,
        user_id: UserId,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>>;
    async fn thread_update(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        patch: ThreadPatch,
    ) -> Result<ThreadVerId>;
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
    async fn user_get(&self, user_id: UserId) -> Result<User>;
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
    // async fn auth_oauth_get(&self, provider: String, user_id: UserId) -> Result<String>;
    async fn auth_oauth_get_remote(&self, provider: String, remote_id: String) -> Result<UserId>;
    async fn auth_oauth_delete(&self, provider: String, user_id: UserId) -> Result<()>;
}

#[async_trait]
pub trait DataSearch {
    async fn search_message(
        &self,
        user_id: UserId,
        query: SearchMessageRequest,
        paginate: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>>;
}

#[async_trait]
pub trait DataAuditLogs {
    async fn audit_logs_room_fetch(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogId>,
    ) -> Result<PaginationResponse<AuditLog>>;
    async fn audit_logs_room_append(
        &self,
        room_id: RoomId,
        user_id: UserId,
        reason: Option<String>,
        payload: MessageSync,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataThreadMember {
    /// is a no-op if membership won't change
    async fn thread_member_put(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()>;
    async fn thread_member_patch(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        patch: ThreadMemberPatch,
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
}

#[async_trait]
pub trait DataUrlEmbed {
    async fn url_embed_insert(&self, user_id: UserId, embed: UrlEmbed) -> Result<()>;
    async fn url_embed_find(&self, url: Url, max_age: Duration) -> Result<Option<UrlEmbed>>;
    async fn url_embed_link(
        &self,
        version_id: MessageVerId,
        embed_id: UrlEmbedId,
        ordering: u32,
    ) -> Result<()>;
}

#[async_trait]
pub trait DataDm {
    async fn dm_put(&self, user_a_id: UserId, user_b_id: UserId, room_id: RoomId) -> Result<()>;
    async fn dm_get(&self, user_a_id: UserId, user_b_id: UserId) -> Result<RoomId>;
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
    async fn user_config_set(&self, user_id: UserId, config: &UserConfig) -> Result<()>;
    async fn user_config_get(&self, user_id: UserId) -> Result<UserConfig>;
}

#[async_trait]
pub trait DataReaction {
    async fn reaction_message_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_message_delete(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_message_list(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>>;
    async fn reaction_message_purge(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
    ) -> Result<()>;
    async fn reaction_thread_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_thread_delete(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    ) -> Result<()>;
    async fn reaction_thread_list(
        &self,
        thread_id: ThreadId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>>;
    async fn reaction_thread_purge(&self, thread_id: ThreadId) -> Result<()>;
}
