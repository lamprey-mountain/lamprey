use im::HashMap as ImMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use common::v1::types::error::ErrorCode;
use common::v1::types::{MessageSync, RoomId, User, UserId};
use tokio::sync::{mpsc, watch};
use tracing::{error, info, trace};

use super::{
    CachedPermissionOverwrite, CachedRole, CachedRoomMember, CachedThread, RoomCommand, RoomData,
    RoomHandle, RoomSnapshot,
};
use crate::consts::{IDLE_TIMEOUT_ROOM, ROOM_ACTOR_MESSAGE_BUDGET};
use crate::services::member_lists::actor::MemberList;
use crate::services::member_lists::util::MemberListKey;
use crate::types::PermissionBits;
use crate::{Error, Result, ServerStateInner};

/// The internal state of a room actor.
pub struct RoomActor {
    state: Arc<ServerStateInner>,
    room_id: RoomId,
    snapshot: Arc<RoomSnapshot>,
    rx: mpsc::Receiver<RoomCommand>,
    snapshot_tx: watch::Sender<Arc<RoomSnapshot>>,
    member_lists: HashMap<MemberListKey, MemberList>,
    last_active: Instant,
}

impl RoomActor {
    pub fn spawn(room_id: RoomId, state: Arc<ServerStateInner>) -> RoomHandle {
        let (tx, rx) = mpsc::channel(1024);
        let snapshot = Arc::new(RoomSnapshot::Loading);
        let (snapshot_tx, snapshot_rx) = watch::channel(snapshot.clone());

        let actor = Self {
            state: state.clone(),
            room_id,
            snapshot,
            rx,
            snapshot_tx,
            member_lists: HashMap::new(),
            last_active: Instant::now(),
        };

        tokio::spawn(async move {
            if let Err(e) = actor.run_supervised().await {
                error!(?room_id, "Room actor supervisor failed: {:?}", e);
            }
        });

        RoomHandle {
            room_id,
            tx,
            snapshot_rx,
        }
    }

    async fn run_supervised(mut self) -> Result<()> {
        let mut backoff = Duration::from_millis(100);

        loop {
            let res = self.run().await;

            match res {
                Ok(()) => {
                    info!(room_id = ?self.room_id, "Room actor shut down gracefully");
                    break;
                }
                Err(e) => {
                    error!(room_id = ?self.room_id, "Room actor error: {:?}. Restarting...", e);
                    if let Error::ApiError(ae) = &e {
                        if ae.code == ErrorCode::UnknownRoom {
                            break;
                        }
                    }
                }
            }

            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(30));
        }

        // Cleanup: unregister all members from the cache
        if let RoomSnapshot::Ready(data) = self.snapshot.as_ref() {
            let rooms = self.state.services().rooms.clone();
            for user_id in data.members.keys() {
                rooms.member_unregister(*user_id, self.room_id);
            }
        }

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        if let Err(e) = self.load_initial_state().await {
            if let Error::ApiError(ae) = &e {
                if ae.code == ErrorCode::UnknownRoom {
                    self.snapshot = Arc::new(RoomSnapshot::NotFound);
                    let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));
                    return Ok(());
                }
            }
            return Err(e);
        }

        let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));

        let mut idle_timeout = tokio::time::interval(Duration::from_secs(60));
        idle_timeout.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut processed = 0;
        loop {
            tokio::select! {
                _ = idle_timeout.tick() => {
                    if self.last_active.elapsed().as_secs() >= IDLE_TIMEOUT_ROOM {
                        info!(room_id = ?self.room_id, "Room actor idle for too long, shutting down");
                        return Ok(());
                    }

                    // Remove idle member lists
                    self.member_lists.retain(|key, list| {
                        if list.is_idle() {
                            trace!(room_id = ?self.room_id, ?key, "Removing idle member list");
                            false
                        } else {
                            true
                        }
                    });
                }
                cmd_opt = self.rx.recv() => {
                    let Some(cmd) = cmd_opt else { return Ok(()) };
                    if !self.handle_command_internal(cmd).await? {
                        return Ok(());
                    }

                    processed += 1;
                    if processed >= ROOM_ACTOR_MESSAGE_BUDGET {
                        tokio::task::yield_now().await;
                        processed = 0;
                    }
                }
            }
        }
    }

    async fn handle_command_internal(&mut self, cmd: RoomCommand) -> Result<bool> {
        self.last_active = Instant::now();
        match cmd {
            RoomCommand::Sync(msg) => {
                if matches!(msg, MessageSync::RoomDelete { room_id } if room_id == self.room_id) {
                    self.snapshot = Arc::new(RoomSnapshot::NotFound);
                    let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));
                    return Ok(false);
                }
                self.handle_sync(msg).await?;
            }
            RoomCommand::MemberList(key, cmd) => {
                if let Some(list) = self.member_lists.get_mut(&key) {
                    list.handle_command(cmd, &self.snapshot).await;
                }
            }
            RoomCommand::MemberListSubscribe(key, events_tx) => {
                if !self.member_lists.contains_key(&key) {
                    if self.snapshot.is_without_members() {
                        self.load_members().await?;
                    }
                    let mut list = MemberList::new(self.state.clone(), key.clone(), events_tx);
                    let _ = list.initialize(Arc::clone(&self.snapshot)).await;
                    self.member_lists.insert(key, list);
                }
            }
            RoomCommand::EnsureMembers => {
                if self.snapshot.is_without_members() {
                    self.load_members().await?;
                }
            }
            RoomCommand::Close => return Ok(false),
        }
        let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));
        Ok(true)
    }

    /// Load members for a room that is in WithoutMembers state.
    /// Transitions the snapshot from WithoutMembers to Ready.
    async fn load_members(&mut self) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        // Get the current snapshot data (without members)
        let current_data = match self.snapshot.as_ref() {
            RoomSnapshot::WithoutMembers(data) | RoomSnapshot::Ready(data) => data.as_ref().clone(),
            _ => return Ok(()), // Can't load members if we're in Loading/NotFound/Unavailable state
        };

        // Load members from database
        let room_members = data.room_member_list_all(self.room_id).await?;

        let user_ids: Vec<_> = room_members.iter().map(|m| m.user_id).collect();
        let users = srv.users.get_many(&user_ids).await?;
        let users_map: HashMap<UserId, Arc<User>> =
            users.into_iter().map(|u| (u.id, Arc::new(u))).collect();

        let mut members = ImMap::new();
        let rooms_srv = srv.rooms.clone();
        for member in room_members {
            let user_id = member.user_id;
            if let Some(user) = users_map.get(&user_id) {
                members.insert(
                    user_id,
                    CachedRoomMember {
                        member,
                        user: user.clone(),
                    },
                );
                rooms_srv.member_register(user_id, self.room_id);
            }
        }

        // Create new snapshot with members loaded
        let new_data = Arc::new(RoomData {
            members,
            ..current_data
        });

        self.snapshot = Arc::new(RoomSnapshot::Ready(new_data));

        Ok(())
    }

    async fn load_initial_state(&mut self) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let (room, room_members, roles_data, channels_data, active_threads_vec) = tokio::try_join!(
            data.room_get(self.room_id),
            data.room_member_list_all(self.room_id),
            data.role_list(
                self.room_id,
                crate::types::PaginationQuery {
                    limit: Some(1024),
                    ..Default::default()
                },
            ),
            data.channel_list(self.room_id),
            data.thread_all_active_room(self.room_id),
        )?;

        let roles_data = roles_data.items;

        let user_ids: Vec<_> = room_members.iter().map(|m| m.user_id).collect();
        let users = srv.users.get_many(&user_ids).await?;
        let users_map: HashMap<UserId, Arc<User>> =
            users.into_iter().map(|u| (u.id, Arc::new(u))).collect();

        let mut members = ImMap::new();
        let rooms_srv = srv.rooms.clone();
        for member in room_members {
            let user_id = member.user_id;
            if let Some(user) = users_map.get(&user_id) {
                members.insert(
                    user_id,
                    CachedRoomMember {
                        member,
                        user: user.clone(),
                    },
                );
                rooms_srv.member_register(user_id, self.room_id);
            }
        }

        let mut roles = ImMap::new();
        for role in roles_data {
            let allow = PermissionBits::from(&role.allow);
            let deny = PermissionBits::from(&role.deny);
            roles.insert(
                role.id,
                CachedRole {
                    inner: role,
                    allow,
                    deny,
                },
            );
        }

        let mut channels = ImMap::new();
        for channel in channels_data {
            if channel.is_thread() {
                continue;
            }
            let mut overwrites = ImMap::new();
            for ow in &channel.permission_overwrites {
                overwrites.insert(
                    ow.id,
                    CachedPermissionOverwrite {
                        id: ow.id,
                        ty: ow.ty,
                        allow: PermissionBits::from(&ow.allow),
                        deny: PermissionBits::from(&ow.deny),
                    },
                );
            }
            channels.insert(
                channel.id,
                super::CachedChannel {
                    inner: channel,
                    overwrites,
                },
            );
        }

        let mut threads = ImMap::new();
        let thread_member_futs = active_threads_vec
            .iter()
            .map(|t| data.thread_member_list_all(t.id));
        let all_thread_members = futures::future::try_join_all(thread_member_futs).await?;

        for (thread, thread_members_vec) in active_threads_vec.into_iter().zip(all_thread_members) {
            let mut members_map = ImMap::new();
            for member in thread_members_vec {
                members_map.insert(member.user_id, member);
            }
            threads.insert(
                thread.id,
                CachedThread {
                    thread,
                    members: members_map,
                },
            );
        }

        self.snapshot = Arc::new(RoomSnapshot::Ready(Arc::new(RoomData {
            room,
            members,
            channels,
            roles,
            threads,
        })));

        Ok(())
    }

    async fn handle_sync(&mut self, event: MessageSync) -> Result<()> {
        let mut snapshot_data = match self.snapshot.as_ref() {
            RoomSnapshot::Ready(data) | RoomSnapshot::WithoutMembers(data) => data.as_ref().clone(),
            _ => return Ok(()), // Don't process sync if not ready
        };

        match &event {
            MessageSync::RoomUpdate { room } => {
                snapshot_data.room = room.clone();
            }
            MessageSync::ChannelCreate { channel } => {
                if channel.room_id != Some(self.room_id) {
                    return Ok(());
                }
                let mut overwrites = ImMap::new();
                for ow in &channel.permission_overwrites {
                    overwrites.insert(
                        ow.id,
                        CachedPermissionOverwrite {
                            id: ow.id,
                            ty: ow.ty,
                            allow: PermissionBits::from(&ow.allow),
                            deny: PermissionBits::from(&ow.deny),
                        },
                    );
                }
                if channel.is_thread() {
                    snapshot_data.threads.insert(
                        channel.id,
                        CachedThread {
                            thread: *channel.clone(),
                            members: ImMap::new(),
                        },
                    );
                } else {
                    snapshot_data.channels.insert(
                        channel.id,
                        super::CachedChannel {
                            inner: *channel.clone(),
                            overwrites,
                        },
                    );
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                if channel.room_id != Some(self.room_id) {
                    return Ok(());
                }
                let mut overwrites = ImMap::new();
                for ow in &channel.permission_overwrites {
                    overwrites.insert(
                        ow.id,
                        CachedPermissionOverwrite {
                            id: ow.id,
                            ty: ow.ty,
                            allow: PermissionBits::from(&ow.allow),
                            deny: PermissionBits::from(&ow.deny),
                        },
                    );
                }
                if channel.is_thread() {
                    if channel.is_removed() {
                        snapshot_data.threads.remove(&channel.id);
                    } else {
                        snapshot_data
                            .threads
                            .entry(channel.id)
                            .and_modify(|t| {
                                t.thread = *channel.clone();
                            })
                            .or_insert_with(|| CachedThread {
                                thread: *channel.clone(),
                                members: ImMap::new(),
                            });
                    }
                } else if channel.is_removed() {
                    snapshot_data.channels.remove(&channel.id);
                } else {
                    snapshot_data.channels.insert(
                        channel.id,
                        super::CachedChannel {
                            inner: *channel.clone(),
                            overwrites,
                        },
                    );
                }
            }
            MessageSync::RoleCreate { role } => {
                if role.room_id != self.room_id {
                    return Ok(());
                }
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                snapshot_data.roles.insert(
                    role.id,
                    CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    },
                );
            }
            MessageSync::RoleUpdate { role } => {
                if role.room_id != self.room_id {
                    return Ok(());
                }
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                snapshot_data.roles.insert(
                    role.id,
                    CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    },
                );
            }
            MessageSync::RoleDelete { role_id, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                snapshot_data.roles.remove(role_id);

                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    // Efficiently update the immutable map using structural sharing
                    for (user_id, member) in snapshot_data.members.clone().into_iter() {
                        if member.member.roles.contains(role_id) {
                            let mut updated_member = member.clone();
                            updated_member.member.roles.retain(|r| r != role_id);
                            snapshot_data.members.insert(user_id, updated_member);
                        }
                    }
                }
            }
            MessageSync::RoleReorder { roles, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                for item in roles {
                    if let Some(mut role) = snapshot_data.roles.get(&item.role_id).cloned() {
                        role.inner.position = item.position;
                        snapshot_data.roles.insert(item.role_id, role);
                    }
                }
            }
            MessageSync::RoomMemberCreate { member, user } => {
                if member.room_id != self.room_id {
                    return Ok(());
                }

                // If we are in WithoutMembers state, we don't insert into members map
                // but we should still update the counts.
                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    if !snapshot_data.members.contains_key(&member.user_id) {
                        snapshot_data.room.member_count += 1;
                        if user.presence.status.is_online() {
                            snapshot_data.room.online_count += 1;
                        }
                    }
                    snapshot_data.members.insert(
                        member.user_id,
                        CachedRoomMember {
                            member: member.clone(),
                            user: Arc::new(user.clone()),
                        },
                    );
                } else {
                    // Just update counts
                    snapshot_data.room.member_count += 1;
                    if user.presence.status.is_online() {
                        snapshot_data.room.online_count += 1;
                    }
                }

                self.state
                    .services()
                    .rooms
                    .member_register(member.user_id, self.room_id);
            }
            MessageSync::RoomMemberUpdate { member, user } => {
                if member.room_id != self.room_id {
                    return Ok(());
                }

                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    if let Some(old_member) = snapshot_data.members.get(&member.user_id) {
                        let old_online = old_member.user.presence.status.is_online();
                        let new_online = user.presence.status.is_online();
                        if old_online != new_online {
                            if new_online {
                                snapshot_data.room.online_count += 1;
                            } else {
                                snapshot_data.room.online_count =
                                    snapshot_data.room.online_count.saturating_sub(1);
                            }
                        }
                    }
                    snapshot_data.members.insert(
                        member.user_id,
                        CachedRoomMember {
                            member: member.clone(),
                            user: Arc::new(user.clone()),
                        },
                    );
                } else {
                    // In WithoutMembers we don't know the old presence state easily
                    // unless we want to query DB or presence service.
                    // For now, let's assume member count doesn't change,
                    // but online count might.
                    // This is a trade-off of lazy loading.
                }

                self.state
                    .services()
                    .rooms
                    .member_register(member.user_id, self.room_id);
            }
            MessageSync::RoomMemberDelete { user_id, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }

                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    if let Some(member) = snapshot_data.members.remove(user_id) {
                        snapshot_data.room.member_count =
                            snapshot_data.room.member_count.saturating_sub(1);
                        if member.user.presence.status.is_online() {
                            snapshot_data.room.online_count =
                                snapshot_data.room.online_count.saturating_sub(1);
                        }
                    }
                } else {
                    snapshot_data.room.member_count =
                        snapshot_data.room.member_count.saturating_sub(1);
                }

                self.state
                    .services()
                    .rooms
                    .member_unregister(*user_id, self.room_id);
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    if let Some(mut member) = snapshot_data.members.get(user_id).cloned() {
                        let old_online = member.user.presence.status.is_online();
                        let new_online = presence.status.is_online();
                        if old_online != new_online {
                            if new_online {
                                snapshot_data.room.online_count += 1;
                            } else {
                                snapshot_data.room.online_count =
                                    snapshot_data.room.online_count.saturating_sub(1);
                            }
                        }

                        member.user = Arc::new({
                            let mut u = member.user.as_ref().clone();
                            u.presence = presence.clone();
                            u
                        });

                        snapshot_data.members.insert(*user_id, member);
                    }
                }
            }
            MessageSync::UserUpdate { user } => {
                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    if let Some(mut member) = snapshot_data.members.get(&user.id).cloned() {
                        let old_online = member.user.presence.status.is_online();
                        let new_online = user.presence.status.is_online();
                        if old_online != new_online {
                            if new_online {
                                snapshot_data.room.online_count += 1;
                            } else {
                                snapshot_data.room.online_count =
                                    snapshot_data.room.online_count.saturating_sub(1);
                            }
                        }

                        member.user = Arc::new(user.clone());

                        snapshot_data.members.insert(user.id, member);
                    }
                }
            }
            MessageSync::MessageCreate { message } | MessageSync::MessageUpdate { message } => {
                if message.room_id != Some(self.room_id) {
                    return Ok(());
                }
                // We don't store messages in RoomSnapshot, but we might need to update member list etc.
            }
            MessageSync::ThreadMemberUpsert {
                thread_id,
                added,
                removed,
                ..
            } => {
                if let Some(thread) = snapshot_data.threads.get_mut(thread_id) {
                    for member in added {
                        thread.members.insert(member.user_id, member.clone());
                    }
                    for user_id in removed {
                        thread.members.remove(user_id);
                    }
                }
            }
            _ => {}
        }

        let new_data = Arc::new(snapshot_data);
        match self.snapshot.as_ref() {
            RoomSnapshot::Ready(_) => {
                self.snapshot = Arc::new(RoomSnapshot::Ready(new_data));
            }
            RoomSnapshot::WithoutMembers(_) => {
                self.snapshot = Arc::new(RoomSnapshot::WithoutMembers(new_data));
            }
            _ => {}
        }

        // Notify member lists of the event
        for list in self.member_lists.values_mut() {
            list.handle_sync(event.clone(), Arc::clone(&self.snapshot))
                .await;
        }

        Ok(())
    }
}
