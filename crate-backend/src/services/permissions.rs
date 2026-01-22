use std::sync::Arc;

use common::v1::types::defaults::EVERYONE_TRUSTED;
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, Permission, PermissionOverwriteType, RoomId, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::error::Result;
use crate::types::Permissions;
use crate::{Error, ServerStateInner};

pub struct ServicePermissions {
    state: Arc<ServerStateInner>,
    cache_perm_room: Cache<(UserId, RoomId), Permissions>,
    cache_perm_channel: Cache<(UserId, RoomId, ChannelId), Permissions>,
    cache_is_mutual: Cache<(UserId, UserId), bool>,
    cache_user_rank: Cache<(RoomId, UserId), u64>,
    timeout_tasks: DashMap<(UserId, RoomId), JoinHandle<()>>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        // not sure what the best way to configure these caches are
        // (userid, roomid) seems a bit inefficient, maybe caching roles would be better
        Self {
            state,
            cache_perm_room: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_perm_channel: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_is_mutual: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_user_rank: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            timeout_tasks: DashMap::new(),
        }
    }

    pub fn purge_cache(&self) {
        self.cache_perm_room.invalidate_all();
        self.cache_perm_channel.invalidate_all();
        self.cache_is_mutual.invalidate_all();
        self.cache_user_rank.invalidate_all();
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
        let data = self.state.data();

        self.cache_perm_room
            .try_get_with((user_id, room_id), async {
                let room = srv.rooms.get(room_id, None).await?;

                // 1. if the user is the owner, they have all permissions
                if room.owner_id == Some(user_id) {
                    let mut p = Permissions::empty();
                    p.add(Permission::ViewChannel);
                    p.add(Permission::Admin);
                    return Result::Ok(p);
                }

                let Ok(member) = data.room_member_get(room_id, user_id).await else {
                    if room.public {
                        // public rooms
                        let mut perms = self.default_for_room(room_id).await?;
                        perms.mask(&[Permission::ViewChannel, Permission::ViewAuditLog]);
                        return Ok(perms);
                    }

                    return Result::Err(Error::NotFound);
                };

                // this handles 2, 3, 4
                let mut perms = data.permission_room_get(user_id, room_id).await?;

                // handle timed out members
                if let Some(timeout_until) = member.timeout_until {
                    if timeout_until > Time::now_utc() {
                        perms.set_timed_out(true);
                    }
                }

                Result::Ok(perms)
            })
            .await
            .map_err(|err| err.fake_clone())
    }

    /// actually calculate the permissions a user has in a channel
    async fn for_channel_inner(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<Permissions> {
        let srv = self.state.services();
        let data = self.state.data();
        let chan = srv.channels.get(channel_id, Some(user_id)).await?;

        // 1. start with parent_id channel (or room) permissions
        let mut perms = if let Some(parent_id) = chan.parent_id {
            Box::pin(self.for_channel(user_id, parent_id)).await?
        } else if let Some(room_id) = chan.room_id {
            self.for_room(user_id, room_id).await?
        } else {
            let mut p = Permissions::empty();
            p.add(Permission::ViewChannel);
            for a in EVERYONE_TRUSTED {
                p.add(*a);
            }

            // permission overwrites dont exist outside of rooms
            return Ok(p);
        };

        // 2. if the user has Admin, return all permissions
        if perms.has(Permission::Admin) {
            return Ok(perms);
        }

        // NOTE: fix this when threads in dms/gdms are implemented
        let room_id = chan
            .room_id
            .expect("only channels in rooms can have permission overwrites set");
        let member = data.room_member_get(room_id, user_id).await?;
        let roles = member.roles;

        if let Some(locked) = &chan.locked {
            let is_expired = locked.until.is_some_and(|until| until <= Time::now_utc());
            if !is_expired {
                perms.set_channel_locked(true);
                for role_id in &locked.allow_roles {
                    if roles.contains(&(*role_id).into()) {
                        perms.set_locked_bypass(true);
                        break;
                    }
                }
            }
        }

        // 3. add all allow permissions for everyone
        for ow in &chan.permission_overwrites {
            if ow.id != *room_id {
                continue;
            }

            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 4. remove all deny permissions for everyone
        for ow in &chan.permission_overwrites {
            if ow.id != *room_id {
                continue;
            }

            for p in &ow.deny {
                perms.remove(*p);
            }
        }

        // 5. add all allow permissions for roles
        for ow in &chan.permission_overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }

            if !roles.contains(&ow.id.into()) {
                continue;
            }

            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 6. remove all deny permissions for roles
        for ow in &chan.permission_overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }

            if !roles.contains(&ow.id.into()) {
                continue;
            }

            for p in &ow.deny {
                perms.remove(*p);
            }
        }

        // 7. add all allow permissions for users
        for ow in &chan.permission_overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }

            if ow.id != *user_id {
                continue;
            }

            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 8. remove all deny permissions for users
        for ow in &chan.permission_overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }

            if ow.id != *user_id {
                continue;
            }

            for p in &ow.deny {
                perms.remove(*p);
            }
        }

        Ok(perms)
    }

    /// calculate the permissions a user has in a channel
    pub async fn for_channel(&self, user_id: UserId, thread_id: ChannelId) -> Result<Permissions> {
        let srv = self.state.services();
        let t = srv.channels.get(thread_id, Some(user_id)).await?;

        self.cache_perm_channel
            .try_get_with(
                (
                    user_id,
                    t.room_id.unwrap_or_else(|| Uuid::nil().into()),
                    thread_id,
                ),
                self.for_channel_inner(user_id, thread_id),
            )
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn invalidate_room(&self, user_id: UserId, room_id: RoomId) {
        self.cache_perm_room.invalidate(&(user_id, room_id)).await;
        self.cache_perm_channel
            .invalidate_entries_if(move |(uid, rid, _), _| room_id == *rid && user_id == *uid)
            .expect("failed to invalidate");
        self.cache_user_rank.invalidate(&(room_id, user_id)).await;
    }

    // NOTE: might be a good idea to be able to invalidate per role
    pub fn invalidate_room_all(&self, room_id: RoomId) {
        self.cache_perm_room
            .invalidate_entries_if(move |(_, rid), _| room_id == *rid)
            .expect("failed to invalidate");
        self.cache_perm_channel
            .invalidate_entries_if(move |(_, rid, _), _| room_id == *rid)
            .expect("failed to invalidate");
        self.cache_user_rank
            .invalidate_entries_if(move |(rid, _), _| room_id == *rid)
            .expect("failed to invalidate");
    }

    pub fn invalidate_thread(&self, user_id: UserId, thread_id: ChannelId) {
        self.cache_perm_channel
            .invalidate_entries_if(move |(uid, _, tid), _| thread_id == *tid && user_id == *uid)
            .expect("failed to invalidate");
    }

    pub fn invalidate_user_ranks(&self, room_id: RoomId) {
        self.cache_user_rank
            .invalidate_entries_if(move |(rid, _), _| room_id == *rid)
            .expect("failed to invalidate");
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
        self.cache_perm_channel
            .invalidate_entries_if(move |(_, _, tid), _| thread_id == *tid)
            .expect("failed to invalidate");

        if let Ok(t) = self.state.services().channels.get(thread_id, None).await {
            if let Some(room_id) = t.room_id {
                self.invalidate_room_all(room_id);
            }
        }
    }

    pub async fn get_user_rank(&self, room_id: RoomId, user_id: UserId) -> Result<u64> {
        self.cache_user_rank
            .try_get_with((room_id, user_id), async {
                let d = self.state.data();
                let rank = d.role_user_rank(room_id, user_id).await?;
                Result::Ok(rank)
            })
            .await
            .map_err(|err| err.fake_clone())
    }

    /// get default permissions for the @everyone role
    ///
    /// for public room joining
    pub async fn default_for_room(&self, room_id: RoomId) -> Result<Permissions> {
        let data = self.state.data();

        let everyone_role_id = room_id.into_inner().into();
        let role = data.role_select(room_id, everyone_role_id).await?;
        let mut perms = Permissions::empty();
        for p in role.allow {
            perms.add(p);
        }
        for p in role.deny {
            perms.remove(p);
        }

        Ok(perms)
    }
}
