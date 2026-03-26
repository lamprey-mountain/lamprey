use std::sync::Arc;

use common::v1::types::defaults::EVERYONE_TRUSTED;
use common::v1::types::util::Time;
use common::v1::types::{
    ChannelId, ConnectionId, Permission, PermissionOverwriteType, RoomId, Session, UserId,
    SERVER_ROOM_ID,
};
use dashmap::DashMap;
use lamprey_backend_core::types::permission::{
    CheckVisibility, MemberState, PermissionBits, Permissions, Permissions2, Permissions2Metadata,
    PermissionsFlags, ResourceContext,
};
use moka::future::Cache;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::error::Result;
use crate::sync::permissions::AuthCheck;
use crate::ServerStateInner;

// TODO(#995): remove much of this code?
// the permission cache can take up a lot of memory and is unnecessary since i can recalculate when needed, thanks to the new cache system
// cache_perm_room, cache_perm_channel, cache_user_rank, and timeout_tasks can probably be removed entirely
// cache_is_mutual *might* be better in ServiceUsers, and mutual could have more data

pub struct ServicePermissions {
    state: Arc<ServerStateInner>,
    cache_is_mutual: Cache<(UserId, UserId), bool>,
    timeout_tasks: DashMap<(UserId, RoomId), JoinHandle<()>>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        // not sure what the best way to configure these caches are
        // (userid, roomid) seems a bit inefficient, maybe caching roles would be better
        Self {
            state,
            cache_is_mutual: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            timeout_tasks: DashMap::new(),
        }
    }

    pub fn purge_cache(&self) {
        self.cache_is_mutual.invalidate_all();
    }

    pub async fn update_timeout_task(
        &self,
        user_id: UserId,
        room_id: RoomId,
        timeout_until: Option<Time>,
    ) {
        if let Some(task) = self.timeout_tasks.remove(&(user_id, room_id)) {
            task.1.abort();
        }

        if let Some(timeout_until) = timeout_until {
            if timeout_until > Time::now_utc() {
                let state = self.state.clone();
                let handle = tokio::spawn(async move {
                    let duration = (timeout_until.into_inner() - Time::now_utc().into_inner())
                        .try_into()
                        .unwrap_or_default();
                    tokio::time::sleep(duration).await;
                    state
                        .services()
                        .perms
                        .invalidate_room(user_id, room_id)
                        .await;
                });
                self.timeout_tasks.insert((user_id, room_id), handle);
            }
        }
    }

    /// calculate the permissions a user has in a room
    pub async fn for_room(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        let srv = self.state.services();
        let calc = srv.cache.permissions(room_id, true).await?;
        let perms2 = calc.query(user_id, None)?;
        Ok(perms2.into())
    }

    pub async fn for_room2(&self, user_id: Option<UserId>, room_id: RoomId) -> Result<Permissions> {
        let srv = self.state.services();
        let calc = srv.cache.permissions(room_id, true).await?;
        let perms2 = calc.query2(user_id, None)?;
        Ok(perms2.into())
    }

    /// calculate the permissions a user has on this server
    pub async fn for_server(&self, user_id: UserId) -> Result<Permissions> {
        self.for_room(user_id, SERVER_ROOM_ID).await
    }

    /// actually calculate the permissions a user has in a channel
    async fn for_channel_inner(
        &self,
        user_id: Option<UserId>,
        channel_id: ChannelId,
    ) -> Result<Permissions2<CheckVisibility>> {
        let srv = self.state.services();
        let chan = srv.channels.get(channel_id, user_id).await?;

        if let Some(room_id) = chan.room_id {
            let calc = srv.cache.permissions(room_id, true).await?;
            let mut perms = calc.query2(user_id, Some(&chan))?;

            // load slowmode fields
            if let Some(uid) = user_id {
                let data = self.state.data();

                if let Some(expire_at) = data
                    .channel_get_message_slowmode_expire_at(channel_id, uid)
                    .await?
                {
                    if expire_at > Time::now_utc() {
                        perms.metadata.channel_slowmode_message_active = true;
                    }
                }

                if chan.is_thread() {
                    if let Some(expire_at) = data
                        .channel_get_thread_slowmode_expire_at(channel_id, uid)
                        .await?
                    {
                        if expire_at > Time::now_utc() {
                            perms.metadata.channel_slowmode_thread_active = true;
                        }
                    }
                }
            }

            Ok(perms.into())
        } else {
            if let Some(parent_id) = chan.parent_id {
                // handle threads in dm/gdms
                Box::pin(self.for_channel_inner(user_id, parent_id)).await
            } else {
                let is_recipient = chan.recipients.iter().any(|r| Some(r.id) == user_id);
                let is_owner = chan.owner_id == user_id;

                let mut bits = PermissionBits::default();
                let mut flags = PermissionsFlags::new();

                if is_recipient || is_owner {
                    bits = PermissionBits::from_slice(EVERYONE_TRUSTED);
                    bits.add(Permission::ChannelView);
                } else {
                    flags.set_cannot_view();
                }

                Ok(Permissions2 {
                    visible: flags.can_view(),
                    context: ResourceContext::Channel(None, channel_id),
                    bits,
                    metadata: Permissions2Metadata {
                        rank: 0,
                        member_state: MemberState::Lurker,
                        channel_locked: false,
                        channel_slowmode_thread_active: false,
                        channel_slowmode_message_active: false,
                    },
                    state: CheckVisibility,
                })
            }
        }
    }

    /// calculate the permissions a user has in a channel
    pub async fn for_channel(&self, user_id: UserId, channel_id: ChannelId) -> Result<Permissions> {
        // why is this wrapped?
        self.for_channel_inner(Some(user_id), channel_id)
            .await
            .map(|p| p.into())
    }

    pub async fn for_channel2(
        &self,
        user_id: Option<UserId>,
        channel_id: ChannelId,
    ) -> Result<Permissions> {
        self.for_channel_inner(user_id, channel_id)
            .await
            .map(|p| p.into())
    }

    pub async fn for_channel3(
        &self,
        user_id: Option<UserId>,
        channel_id: ChannelId,
    ) -> Result<Permissions2<CheckVisibility>> {
        self.for_channel_inner(user_id, channel_id).await
    }

    pub async fn invalidate_room(&self, user_id: UserId, room_id: RoomId) {
        let _ = self
            .state
            .services()
            .cache
            .reload_member(room_id, user_id)
            .await;
        // Permission caches removed - permissions are recalculated on-demand
    }

    // NOTE: might be a good idea to be able to invalidate per role
    pub async fn invalidate_room_all(&self, _room_id: RoomId) {
        // Permission caches removed - permissions are recalculated on-demand
    }

    pub async fn invalidate_thread(&self, _user_id: UserId, thread_id: ChannelId) {
        if let Ok(c) = self.state.services().channels.get(thread_id, None).await {
            if let Some(rid) = c.room_id {
                let _ = self
                    .state
                    .services()
                    .cache
                    .reload_channel(rid, thread_id)
                    .await;
            }
        }
        // Permission caches removed - permissions are recalculated on-demand
    }

    pub fn invalidate_user_ranks(&self, _room_id: RoomId) {
        // Rank cache removed - ranks are recalculated on-demand
    }

    /// check if two users share a common room
    pub async fn is_mutual(&self, a: UserId, b: UserId) -> Result<bool> {
        if a == b {
            return Ok(true);
        }
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        let data = self.state.data();
        self.cache_is_mutual
            .try_get_with((a, b), data.permission_is_mutual(a, b))
            .await
            .map_err(|err| err.fake_clone())
    }

    pub fn invalidate_is_mutual(&self, user_id: UserId) {
        self.cache_is_mutual
            .invalidate_entries_if(move |(a, b), _| *a == user_id || *b == user_id)
            .expect("failed to invalidate");
    }

    pub async fn permission_overwrite_upsert(
        &self,
        thread_id: ChannelId,
        overwrite_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    ) -> Result<()> {
        let data = self.state.data();
        data.permission_overwrite_upsert(thread_id.into(), overwrite_id, ty, allow, deny)
            .await?;

        // Invalidate caches
        self.invalidate_thread_all(thread_id).await;
        Ok(())
    }

    pub async fn permission_overwrite_delete(
        &self,
        thread_id: ChannelId,
        overwrite_id: Uuid,
    ) -> Result<()> {
        let data = self.state.data();
        data.permission_overwrite_delete(thread_id, overwrite_id)
            .await?;

        // Invalidate caches
        self.invalidate_thread_all(thread_id).await;
        Ok(())
    }

    async fn invalidate_thread_all(&self, thread_id: ChannelId) {
        // Permission caches removed - permissions are recalculated on-demand

        if let Ok(t) = self.state.services().channels.get(thread_id, None).await {
            if let Some(room_id) = t.room_id {
                let _ = self
                    .state
                    .services()
                    .cache
                    .reload_channel(room_id, thread_id)
                    .await;
                self.invalidate_room_all(room_id).await;
            }
        }
    }

    pub async fn get_user_rank(&self, room_id: RoomId, user_id: UserId) -> Result<u64> {
        let srv = self.state.services();
        let calc = srv.cache.permissions(room_id, true).await?;
        Ok(calc.rank(user_id))
    }

    /// get default permissions for the @everyone role
    ///
    /// for public room joining
    // TODO: move to permissions cache
    pub async fn default_for_room(&self, room_id: RoomId) -> Result<Permissions> {
        let data = self.state.data();

        let everyone_role_id = room_id.into_inner().into();
        let role = data.role_select(room_id, everyone_role_id).await?;
        use lamprey_backend_core::types::permission::PermissionBits;
        let mut perms = PermissionBits::default();
        perms.add_all(role.allow.into());
        perms.remove_all(role.deny.into());

        // Convert to Permissions2
        let mut p = Permissions::builder();
        p.perms = perms;
        Ok(p.build())
    }

    /// Check if target user allows DMs from source user
    pub async fn allows_dm_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool> {
        let data = self.state.data();
        data.permission_allows_dm_from_user(source_user_id, target_user_id)
            .await
    }

    /// Check if target user allows friend requests from source user
    pub async fn allows_friend_request_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool> {
        let data = self.state.data();
        data.permission_allows_friend_request_from_user(source_user_id, target_user_id)
            .await
    }

    pub async fn auth_check(
        &self,
        auth_check: &AuthCheck,
        session: &Session,
        connection_id: ConnectionId,
    ) -> Result<bool> {
        let uid = session.user_id();
        let should_send = match auth_check {
            AuthCheck::Room(room_id) => {
                // Use can_view_room directly on calculator for efficiency
                let srv = self.state.services();
                let perms_calc = srv.cache.permissions(*room_id, true).await?;
                perms_calc.can_view_room(uid)
            }
            AuthCheck::RoomPerm(room_id, perm) => {
                // Use service method that returns old Permissions for compatibility
                self.for_room2(uid, *room_id).await?.has(*perm)
            }
            AuthCheck::Channel(channel_id) => {
                let perms = self.for_channel2(uid, *channel_id).await?;
                perms.has(Permission::ChannelView)
            }
            AuthCheck::ChannelPerm(channel_id, perm) => {
                let perms = self.for_channel2(uid, *channel_id).await?;
                perms.has(*perm)
            }
            AuthCheck::User(target_user_id) => {
                if let Some(user_id) = session.user_id() {
                    user_id == *target_user_id
                } else {
                    false
                }
            }
            AuthCheck::UserVisible(target_user_id) => {
                if let Some(user_id) = session.user_id() {
                    if user_id == *target_user_id {
                        true
                    } else {
                        self.is_mutual(user_id, *target_user_id).await?
                    }
                } else {
                    false
                }
            }
            AuthCheck::Session(session_id) => session.id == *session_id,
            AuthCheck::Connection(target_conn_id) => connection_id == *target_conn_id,
            AuthCheck::Any(checks) => {
                // PERF: optimize; two `AuthCheck::Room`s should only fetch the room once
                for check in checks {
                    if Box::pin(self.auth_check(check, session, connection_id)).await? {
                        return Ok(true);
                    }
                }
                false
            }
        };

        Ok(should_send)
    }
}
