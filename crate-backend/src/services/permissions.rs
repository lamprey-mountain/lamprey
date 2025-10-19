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

    pub async fn for_room(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        let srv = self.state.services();
        let data = self.state.data();

        self.cache_perm_room
            .try_get_with((user_id, room_id), async {
                let room = srv.rooms.get(room_id, None).await?;
                if room.owner_id == Some(user_id) {
                    let mut p = Permissions::empty();
                    p.add(Permission::ViewChannel);
                    p.add(Permission::Admin);
                    return Result::Ok(p);
                }

                let mut perms = data.permission_room_get(user_id, room_id).await?;
                if let Ok(member) = data.room_member_get(room_id, user_id).await {
                    if let Some(timeout_until) = member.timeout_until {
                        if timeout_until > Time::now_utc() {
                            perms.set_timed_out(true);
                        }
                    }
                } else {
                    return Result::Err(Error::NotFound);
                }

                Result::Ok(perms)
            })
            .await
            .map_err(|err| err.fake_clone())
    }

    async fn for_thread_inner(&self, user_id: UserId, thread_id: ChannelId) -> Result<Permissions> {
        let srv = self.state.services();
        let data = self.state.data();
        let thread = srv.channels.get(thread_id, Some(user_id)).await?;
        let (mut perms, roles) = if let Some(room_id) = thread.room_id {
            let room = srv.rooms.get(room_id, None).await?;
            if room.owner_id == Some(user_id) {
                let mut p = Permissions::empty();
                p.add(Permission::ViewChannel);
                p.add(Permission::Admin);
                return Ok(p);
            }

            let member = data.room_member_get(room_id, user_id).await?;
            (self.for_room(user_id, room_id).await?, member.roles)
        } else {
            data.thread_member_get(thread_id, user_id).await?;
            let mut p = Permissions::empty();
            p.add(Permission::ViewChannel);
            for a in EVERYONE_TRUSTED {
                p.add(*a);
            }
            (p, vec![])
        };

        // admins always have full permissions and ignore overwrites
        if !perms.has(Permission::Admin) {
            let mut parents = Vec::new();
            let mut current_parent_id = thread.parent_id;
            while let Some(parent_id) = current_parent_id {
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                current_parent_id = parent.parent_id;
                parents.push(parent);
            }

            // apply overwrites from top-most parent down to the direct parent
            for parent in parents.into_iter().rev() {
                for o in parent.permission_overwrites {
                    if o.ty == PermissionOverwriteType::User {
                        if o.id != *user_id {
                            continue;
                        }
                    } else if o.ty == PermissionOverwriteType::Role {
                        if !roles.contains(&o.id.into()) {
                            continue;
                        }
                    }
                    for p in o.allow {
                        perms.add(p);
                    }
                    for p in o.deny {
                        perms.remove(p);
                    }
                }
            }

            for o in thread.permission_overwrites {
                if o.ty == PermissionOverwriteType::User {
                    if o.id != *user_id {
                        continue;
                    }
                } else if o.ty == PermissionOverwriteType::Role {
                    if !roles.contains(&o.id.into()) {
                        continue;
                    }
                }
                for p in o.allow {
                    perms.add(p);
                }
                for p in o.deny {
                    perms.remove(p);
                }
            }
        }

        Ok(perms)
    }

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
                self.for_thread_inner(user_id, thread_id),
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
}
