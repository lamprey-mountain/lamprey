//! Service for managing member lists
//!
//! ## Member list logic
//!
//! In threads, the active member set is all members who are have an associated
//! thread_member object. In other channels, a member is active if they can view
//! the channel.
//!
//! A group is formed for each hoisted role, online members, and offline members.
//! Role groups are returned first (ordered by position), followed by online
//! members, then finally by offline members. A member is part of group formed by
//! their highest hoisted role. Role groups only contain online members, offline
//! members are always part of the offline group regardless of roles. If a group
//! has no members, it is not returned.
//!
//! After the member sets are filtered and grouped, they are ordered by their
//! display name. The display name uses the room override_name, falling back to
//! user name.

use std::sync::Arc;

use common::v1::types::{
    ChannelId, MemberListGroup, MemberListGroupId, MemberListOp, MessageSync, Permission, Role,
    RoomId, RoomMember, ThreadMember, User, UserId,
};
use dashmap::DashMap;
use futures::StreamExt;
use moka::future::Cache;
use tokio::sync::{broadcast, watch, Mutex};
use tracing::error;

use crate::{error::Error, services::members::util::MemberListKey, Result, ServerStateInner};

use self::util::MemberGroup;

mod util;

pub use util::{MemberList, MemberListItem, MemberListTarget};

pub struct ServiceMembers {
    inner: Arc<ServiceMembersInner>,
}

struct ServiceMembersInner {
    state: Arc<ServerStateInner>,
    member_lists: DashMap<MemberListKey, Arc<(MemberList, broadcast::Sender<Vec<MemberListOp>>)>>,
    cache_room_member: Cache<(UserId, RoomId), RoomMember>,
    cache_thread_member: Cache<(UserId, ChannelId), ThreadMember>,
}

pub struct ServiceMembersSyncer {
    inner: Arc<ServiceMembersInner>,
    query_tx: watch::Sender<Option<(MemberListTarget, Vec<(u64, u64)>)>>,
    query_rx: watch::Receiver<Option<(MemberListTarget, Vec<(u64, u64)>)>>,
    user_id: Mutex<Option<UserId>>,
    ops_rx: Mutex<Option<broadcast::Receiver<Vec<MemberListOp>>>>,
    current_target: Mutex<Option<MemberListTarget>>,
}

impl ServiceMembers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let inner = Arc::new(ServiceMembersInner {
            state: state.clone(),
            member_lists: DashMap::new(),
            cache_room_member: Cache::builder().max_capacity(1_000_000).build(),
            cache_thread_member: Cache::builder().max_capacity(1_000_000).build(),
        });

        let inner2 = inner.clone();
        let mut sub = state.sushi.subscribe();
        tokio::spawn(async move {
            while let Ok(msg) = sub.recv().await {
                if let Err(err) = inner2.handle_event(&msg).await {
                    error!("service members error: {err}");
                }
            }
        });

        Self { inner }
    }

    pub fn create_syncer(&self) -> ServiceMembersSyncer {
        ServiceMembersSyncer::new(self.inner.clone())
    }
}

impl ServiceMembersSyncer {
    fn new(inner: Arc<ServiceMembersInner>) -> Self {
        let (tx, rx) = watch::channel(None);
        Self {
            inner,
            query_tx: tx,
            query_rx: rx,
            user_id: Mutex::new(None),
            ops_rx: Mutex::new(None),
            current_target: Mutex::new(None),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<UserId>) {
        *self.user_id.lock().await = user_id;
    }

    /// set the new query
    pub fn set_query(&mut self, target: MemberListTarget, ranges: &[(u64, u64)]) {
        self.query_tx.send(Some((target, ranges.to_vec()))).ok();
    }

    /// poll for the next member list sync message
    pub async fn next(&self) -> Result<MessageSync> {
        let mut query_rx = self.query_rx.clone();
        loop {
            let query_changed = {
                let mut current_target_guard = self.current_target.lock().await;
                let query_target = query_rx.borrow().as_ref().map(|(t, _)| t.clone());

                if query_target.as_ref() != (*current_target_guard).as_ref() {
                    // Mark as changed and clear ops_rx to force re-sync
                    *current_target_guard = query_target;
                    *self.ops_rx.lock().await = None;
                    true
                } else {
                    false
                }
            };

            if query_changed || self.ops_rx.lock().await.is_none() {
                let maybe_query = query_rx.borrow().clone();
                let (target, ranges) = if let Some(q) = maybe_query {
                    q
                } else {
                    query_rx
                        .changed()
                        .await
                        .map_err(|_| Error::BadStatic("syncer closed"))?;
                    continue;
                };
                let user_id = self.user_id.lock().await.ok_or(Error::UnauthSession)?;

                let (list, ops_tx) = &*self.inner.get_or_create_list(&target, user_id).await?;
                let mut ops_rx = ops_tx.subscribe();

                let mut ops = vec![];
                for (start, end) in &ranges {
                    let end = (*end).min(list.sorted_members.len() as u64);
                    if start >= &end {
                        continue;
                    }
                    let slice = &list.sorted_members[*start as usize..end as usize];

                    let full_info = self
                        .inner
                        .populate_full_members_info(&target, slice, Some(user_id))
                        .await?;

                    let mut room_members = vec![];
                    let mut thread_members = vec![];
                    let mut users = vec![];

                    for (rm, tm, u) in full_info {
                        if let Some(rm) = rm {
                            room_members.push(rm);
                        }
                        if let Some(tm) = tm {
                            thread_members.push(tm);
                        }
                        users.push(u);
                    }

                    ops.push(MemberListOp::Sync {
                        position: *start,
                        room_members: if room_members.is_empty() {
                            None
                        } else {
                            Some(room_members)
                        },
                        thread_members: if thread_members.is_empty() {
                            None
                        } else {
                            Some(thread_members)
                        },
                        users,
                    });
                }

                // drain any pending ops
                while let Ok(pending_ops) = ops_rx.try_recv() {
                    ops.extend(pending_ops);
                }

                *self.ops_rx.lock().await = Some(ops_rx);

                return Ok(MessageSync::MemberListSync {
                    user_id,
                    room_id: list.room_id,
                    channel_id: match target {
                        MemberListTarget::Room(_) => None,
                        MemberListTarget::Channel(id) => Some(id),
                    },
                    ops,
                    groups: list.groups.clone(),
                });
            } else {
                let mut ops_rx_guard = self.ops_rx.lock().await;
                if let Some(ops_rx) = ops_rx_guard.as_mut() {
                    tokio::select! {
                        res = query_rx.changed() => {
                            if res.is_err() {
                                return Err(Error::BadStatic("syncer closed"));
                            }
                            // query changed, loop will handle it
                            continue;
                        },
                        res = ops_rx.recv() => {
                            match res {
                                Ok(ops) => {
                                    let user_id = self.user_id.lock().await.ok_or(Error::UnauthSession)?;
                                    let target = self.current_target.lock().await.clone().unwrap();
                                    let (list, _) = &*self.inner.get_or_create_list(&target, user_id).await?;

                                    return Ok(MessageSync::MemberListSync {
                                        user_id,
                                        room_id: list.room_id,
                                        channel_id: match target {
                                            MemberListTarget::Room(_) => None,
                                            MemberListTarget::Channel(id) => Some(id),
                                        },
                                        ops,
                                        groups: list.groups.clone(),
                                    });
                                }
                                Err(broadcast::error::RecvError::Lagged(_)) => {
                                    // Force re-sync
                                    *ops_rx_guard = None;
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    // List was removed, force re-sync which will likely fail if target is gone
                                    *ops_rx_guard = None;
                                }
                            }
                        }
                    }
                } else {
                    // Should be handled by the query_changed block
                    query_rx
                        .changed()
                        .await
                        .map_err(|_| Error::BadStatic("syncer closed"))?;
                }
            }
        }
    }
}

impl ServiceMembersInner {
    async fn get_or_create_list(
        &self,
        target: &MemberListTarget,
        user_id: UserId,
    ) -> Result<Arc<(MemberList, broadcast::Sender<Vec<MemberListOp>>)>> {
        match target {
            MemberListTarget::Room(room_id) => {
                if let Some(list) = self.member_lists.get(&MemberListKey::room(*room_id)) {
                    return Ok(list.clone());
                }

                let list = self.compute_member_list(target, user_id).await?;
                let (tx, _) = broadcast::channel(256);
                let entry = self
                    .member_lists
                    .entry(MemberListKey::room(*room_id))
                    .or_insert(Arc::new((list, tx)));
                Ok(entry.value().clone())
            }
            MemberListTarget::Channel(channel_id) => {
                if let Some(list) = self.member_lists.get(&MemberListKey::channel(*channel_id)) {
                    return Ok(list.clone());
                }

                let list = self.compute_member_list(target, user_id).await?;
                let (tx, _) = broadcast::channel(256);
                let entry = self
                    .member_lists
                    .entry(MemberListKey::channel(*channel_id))
                    .or_insert(Arc::new((list, tx)));
                Ok(entry.value().clone())
            }
        }
    }

    async fn compute_member_list(
        &self,
        target: &MemberListTarget,
        user_id: UserId,
    ) -> Result<MemberList> {
        let (members, room_roles, room_id) =
            self.get_full_member_info_list(target, user_id).await?;
        self.sort_and_group_list(room_id, members, room_roles)
    }

    async fn get_full_member_info_list(
        &self,
        target: &MemberListTarget,
        user_id: UserId,
    ) -> Result<(
        Vec<(Option<RoomMember>, Option<ThreadMember>, User)>,
        Option<Vec<Role>>,
        Option<RoomId>,
    )> {
        let srv = self.state.services();
        let data = self.state.data();

        let (room_id, members, users, room_roles) = match target {
            MemberListTarget::Room(room_id) => {
                let members = data.room_member_list_all(*room_id).await?;
                let user_ids: Vec<_> = members.iter().map(|m| m.user_id).collect();
                let users = futures::future::try_join_all(
                    user_ids
                        .into_iter()
                        .map(|id| srv.users.get(id, Some(user_id))),
                )
                .await?;
                let roles = data.role_list(*room_id, Default::default()).await?.items;
                (Some(*room_id), members, users, Some(roles))
            }
            MemberListTarget::Channel(thread_id) => {
                let thread = srv.channels.get(*thread_id, Some(user_id)).await?;

                if !thread.ty.is_thread() && thread.room_id.is_some() {
                    let room_id = thread.room_id.unwrap();
                    let members = data.room_member_list_all(room_id).await?;

                    let mut fut = futures::stream::FuturesUnordered::new();
                    for member in members {
                        let srv = srv.clone();
                        fut.push(async move {
                            if srv
                                .perms
                                .for_channel(member.user_id, *thread_id)
                                .await
                                .is_ok_and(|p| p.has(Permission::ViewChannel))
                            {
                                Some(member)
                            } else {
                                None
                            }
                        });
                    }
                    let mut visible_members = Vec::new();
                    while let Some(result) = fut.next().await {
                        if let Some(member) = result {
                            visible_members.push(member);
                        }
                    }

                    let user_ids: Vec<_> = visible_members.iter().map(|m| m.user_id).collect();
                    let users = futures::future::try_join_all(
                        user_ids
                            .into_iter()
                            .map(|id| srv.users.get(id, Some(user_id))),
                    )
                    .await?;
                    let roles = data.role_list(room_id, Default::default()).await?.items;
                    (Some(room_id), visible_members, users, Some(roles))
                } else {
                    let thread_members = data.thread_member_list_all(*thread_id).await?;
                    let room_id = thread.room_id;
                    let (room_members, roles) = if let Some(room_id) = room_id {
                        (
                            Some(data.room_member_list_all(room_id).await?),
                            Some(data.role_list(room_id, Default::default()).await?.items),
                        )
                    } else {
                        (None, None)
                    };
                    let user_ids: Vec<_> = thread_members.iter().map(|m| m.user_id).collect();
                    let users = futures::future::try_join_all(
                        user_ids
                            .into_iter()
                            .map(|id| srv.users.get(id, Some(user_id))),
                    )
                    .await?;

                    let room_members_map: std::collections::HashMap<_, _> = room_members
                        .unwrap_or_default()
                        .into_iter()
                        .map(|rm| (rm.user_id, rm))
                        .collect();

                    let members: Vec<_> = thread_members
                        .into_iter()
                        .map(|tm| {
                            let rm = room_members_map.get(&tm.user_id).cloned();
                            (
                                rm,
                                Some(tm.clone()),
                                users.iter().find(|u| u.id == tm.user_id).unwrap().clone(),
                            )
                        })
                        .collect();

                    return Ok((members, roles, room_id));
                }
            }
        };

        let full_members = members
            .into_iter()
            .zip(users.into_iter())
            .map(|(rm, u)| (Some(rm), None, u))
            .collect();

        Ok((full_members, room_roles, room_id))
    }

    fn sort_and_group_list(
        &self,
        room_id: Option<RoomId>,
        mut members: Vec<(Option<RoomMember>, Option<ThreadMember>, User)>,
        room_roles: Option<Vec<Role>>,
    ) -> Result<MemberList> {
        let srv = self.state.services();

        let roles_map: std::collections::HashMap<_, _> = if let Some(roles) = room_roles.as_ref() {
            roles.iter().map(|r| (r.id, r)).collect()
        } else {
            Default::default()
        };

        let get_highest_hoisted_role = |rm: &Option<RoomMember>| -> Option<&Role> {
            rm.as_ref().and_then(|m| {
                m.roles
                    .iter()
                    .filter_map(|role_id| roles_map.get(role_id))
                    .filter(|r| r.hoist)
                    .min_by_key(|r| r.position)
                    .copied()
            })
        };

        let get_group = |rm: &Option<RoomMember>, user: &User| {
            if srv.presence.is_online(user.id) {
                if let Some(role) = get_highest_hoisted_role(rm) {
                    MemberGroup::Hoisted {
                        role_id: role.id,
                        role_position: role.position,
                    }
                } else {
                    MemberGroup::Online
                }
            } else {
                MemberGroup::Offline
            }
        };

        members.sort_by(|(a_rm, _, a_user), (b_rm, _, b_user)| {
            let a_group = get_group(a_rm, a_user);
            let b_group = get_group(b_rm, b_user);

            let a_display_name = a_rm
                .as_ref()
                .and_then(|m| m.override_name.as_deref())
                .unwrap_or(&a_user.name);

            let b_display_name = b_rm
                .as_ref()
                .and_then(|m| m.override_name.as_deref())
                .unwrap_or(&b_user.name);

            a_group
                .cmp(&b_group)
                .then_with(|| a_display_name.cmp(b_display_name))
        });

        let mut groups = vec![];
        if let Some(room_roles) = &room_roles {
            let mut hoisted_roles: Vec<_> = room_roles.iter().filter(|r| r.hoist).collect();
            hoisted_roles.sort_by_key(|r| r.position);

            for role in hoisted_roles {
                let count = members
                    .iter()
                    .filter(|(rm, _, u)| {
                        srv.presence.is_online(u.id)
                            && get_highest_hoisted_role(rm).map_or(false, |r| r.id == role.id)
                    })
                    .count();
                if count > 0 {
                    groups.push(MemberListGroup {
                        id: MemberListGroupId::Role(role.id),
                        count: count as u64,
                    });
                }
            }
        }

        let online_count = members
            .iter()
            .filter(|(_, _, u)| srv.presence.is_online(u.id))
            .count() as u64;
        let offline_count = members.len() as u64 - online_count;

        let online_hoisted_count: u64 = groups.iter().map(|g| g.count).sum();
        let online_unhoisted_count = online_count - online_hoisted_count;

        if online_unhoisted_count > 0 {
            groups.push(MemberListGroup {
                id: MemberListGroupId::Online,
                count: online_unhoisted_count,
            });
        }

        if offline_count > 0 {
            groups.push(MemberListGroup {
                id: MemberListGroupId::Offline,
                count: offline_count,
            });
        }

        let sorted_members = members
            .into_iter()
            .map(|(rm, _tm, u)| {
                let user_id = u.id;
                let display_name = rm.and_then(|m| m.override_name).unwrap_or(u.name);
                MemberListItem {
                    user_id,
                    display_name: display_name.into(),
                }
            })
            .collect();

        Ok(MemberList::new(room_id, sorted_members, groups))
    }

    async fn get_room_member(&self, room_id: RoomId, user_id: UserId) -> Option<RoomMember> {
        self.cache_room_member
            .try_get_with(
                (user_id, room_id),
                self.state.data().room_member_get(room_id, user_id),
            )
            .await
            .ok()
    }

    async fn get_thread_member(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Option<ThreadMember> {
        if let Ok(channel) = self.state.services().channels.get(thread_id, None).await {
            if channel.ty.is_thread() {
                return self
                    .cache_thread_member
                    .try_get_with(
                        (user_id, thread_id),
                        self.state.data().thread_member_get(thread_id, user_id),
                    )
                    .await
                    .ok();
            }
        }
        None
    }

    async fn populate_full_member_info(
        &self,
        target: &MemberListTarget,
        user_id: UserId,
        observer_id: Option<UserId>,
    ) -> Result<(Option<RoomMember>, Option<ThreadMember>, User)> {
        let srv = self.state.services();
        // refetch user with relationships
        let user = srv.users.get(user_id, observer_id).await?;

        let (room_id, channel_id) = match target {
            MemberListTarget::Room(room_id) => (Some(*room_id), None),
            MemberListTarget::Channel(channel_id) => {
                let channel = srv.channels.get(*channel_id, observer_id).await?;
                (channel.room_id, Some(*channel_id))
            }
        };

        let room_member = if let Some(room_id) = room_id {
            self.get_room_member(room_id, user_id).await
        } else {
            None
        };

        let thread_member = if let Some(channel_id) = channel_id {
            self.get_thread_member(channel_id, user_id).await
        } else {
            None
        };

        Ok((room_member, thread_member, user))
    }

    async fn populate_full_members_info(
        &self,
        target: &MemberListTarget,
        items: &[MemberListItem],
        observer_id: Option<UserId>,
    ) -> Result<Vec<(Option<RoomMember>, Option<ThreadMember>, User)>> {
        let mut fut = futures::stream::FuturesUnordered::new();
        for item in items {
            fut.push(self.populate_full_member_info(target, item.user_id, observer_id));
        }

        let mut full_info = Vec::with_capacity(items.len());
        while let Some(result) = fut.next().await {
            full_info.push(result?);
        }

        Ok(full_info)
    }

    pub async fn handle_event(&self, event: &MessageSync) -> Result<()> {
        match event {
            MessageSync::RoomDelete { room_id } => {
                self.remove_room_list(*room_id);
            }
            MessageSync::ChannelUpdate { channel } => {
                self.recalculate_channel_list(channel.id).await?;
            }
            MessageSync::RoomMemberUpsert { member } => {
                self.cache_room_member
                    .insert((member.user_id, member.room_id), member.clone())
                    .await;
                self.recalculate_room_list(member.room_id).await?;
            }
            MessageSync::ThreadMemberUpsert { member } => {
                self.cache_thread_member
                    .insert((member.user_id, member.thread_id), member.clone())
                    .await;
                self.recalculate_channel_list(member.thread_id).await?;
            }
            MessageSync::RoleCreate { role } => {
                self.recalculate_room_list(role.room_id).await?;
            }
            MessageSync::RoleUpdate { role } => {
                self.recalculate_room_list(role.room_id).await?;
            }
            MessageSync::RoleDelete {
                room_id,
                role_id: _,
            } => {
                self.recalculate_room_list(*room_id).await?;
            }
            MessageSync::RoleReorder { room_id, roles: _ } => {
                self.recalculate_room_list(*room_id).await?;
            }
            _ => {}
        };
        Ok(())
    }

    fn remove_room_list(&self, room_id: RoomId) {
        if let Some((_, arc)) = self.member_lists.remove(&MemberListKey::room(room_id)) {
            let (old_list, tx) = &*arc;
            let count = old_list.sorted_members.len() as u64;
            if count > 0 {
                tx.send(vec![MemberListOp::Delete { position: 0, count }])
                    .ok();
            }
        }
    }

    async fn recalculate_channel_list(&self, channel_id: ChannelId) -> Result<()> {
        if let Some(entry) = self.member_lists.get(&MemberListKey::channel(channel_id)) {
            let (old_list, tx) = entry.value().as_ref();
            let new_list = self
                .compute_member_list(&MemberListTarget::Channel(channel_id), UserId::new())
                .await?;

            let old_ids: Vec<_> = old_list.sorted_members.iter().map(|i| i.user_id).collect();
            let new_ids: Vec<_> = new_list.sorted_members.iter().map(|i| i.user_id).collect();

            let diff = diff::slice(&old_ids, &new_ids);

            let mut ops = vec![];

            for result in diff {
                match result {
                    diff::Result::Left(user_id) => {
                        let pos = old_list
                            .sorted_members
                            .iter()
                            .position(|i| i.user_id == *user_id)
                            .unwrap();
                        ops.push(MemberListOp::Delete {
                            position: pos as u64,
                            count: 1,
                        });
                    }
                    diff::Result::Right(user_id) => {
                        let pos = new_list
                            .sorted_members
                            .iter()
                            .position(|i| i.user_id == *user_id)
                            .unwrap();
                        let (room_member, thread_member, user) = self
                            .populate_full_member_info(
                                &MemberListTarget::Channel(channel_id),
                                *user_id,
                                None,
                            )
                            .await?;
                        ops.push(MemberListOp::Insert {
                            position: pos as u64,
                            room_member,
                            thread_member,
                            user: Box::new(user),
                        });
                    }
                    diff::Result::Both(_, _) => {}
                }
            }

            if !ops.is_empty() {
                tx.send(ops).ok();
            }

            self.member_lists.insert(
                MemberListKey::channel(channel_id),
                Arc::new((new_list, tx.clone())),
            );
        }

        Ok(())
    }

    async fn recalculate_room_list(&self, room_id: RoomId) -> Result<()> {
        if let Some(entry) = self.member_lists.get(&MemberListKey::room(room_id)) {
            let (old_list, tx) = entry.value().as_ref();
            let new_list = self
                .compute_member_list(&MemberListTarget::Room(room_id), UserId::new())
                .await?;

            let old_ids: Vec<_> = old_list.sorted_members.iter().map(|i| i.user_id).collect();
            let new_ids: Vec<_> = new_list.sorted_members.iter().map(|i| i.user_id).collect();

            let diff = diff::slice(&old_ids, &new_ids);

            let mut ops = vec![];

            for result in diff {
                match result {
                    diff::Result::Left(user_id) => {
                        let pos = old_list
                            .sorted_members
                            .iter()
                            .position(|i| i.user_id == *user_id)
                            .unwrap();
                        ops.push(MemberListOp::Delete {
                            position: pos as u64,
                            count: 1,
                        });
                    }
                    diff::Result::Right(user_id) => {
                        let pos = new_list
                            .sorted_members
                            .iter()
                            .position(|i| i.user_id == *user_id)
                            .unwrap();
                        let (room_member, thread_member, user) = self
                            .populate_full_member_info(
                                &MemberListTarget::Room(room_id),
                                *user_id,
                                None,
                            )
                            .await?;
                        ops.push(MemberListOp::Insert {
                            position: pos as u64,
                            room_member,
                            thread_member,
                            user: Box::new(user),
                        });
                    }
                    diff::Result::Both(_, _) => {}
                }
            }

            if !ops.is_empty() {
                tx.send(ops).ok();
            }

            self.member_lists.insert(
                MemberListKey::room(room_id),
                Arc::new((new_list, tx.clone())),
            );
        }

        // Invalidate all channel member lists in the room
        let channel_ids_to_invalidate: Vec<ChannelId> = self
            .member_lists
            .iter()
            .filter(|entry| entry.value().0.room_id == Some(room_id))
            .filter_map(|entry| entry.key().channel_id)
            .collect();

        for channel_id in channel_ids_to_invalidate {
            self.recalculate_channel_list(channel_id).await?;
        }

        Ok(())
    }
}
