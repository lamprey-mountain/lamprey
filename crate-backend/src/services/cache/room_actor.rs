use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::v1::types::error::ErrorCode;
use common::v1::types::{MessageSync, RoomId};
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

use crate::services::cache::room::{
    CachedPermissionOverwrite, CachedRole, CachedRoomMember, CachedThread, RoomCommand, RoomHandle,
    RoomSnapshot, RoomStatus,
};
use crate::services::member_lists::actor::MemberList;
use crate::services::member_lists::util::MemberListKey;
use crate::types::PermissionBits;
use crate::{Error, Result, ServerStateInner};

/// The internal state of a room actor.
pub struct RoomActor {
    state: Arc<ServerStateInner>,
    room_id: RoomId,
    snapshot: RoomSnapshot,
    rx: mpsc::Receiver<RoomCommand>,
    snap_tx: watch::Sender<Arc<RoomSnapshot>>,
    member_lists: HashMap<MemberListKey, MemberList>,
}

impl RoomActor {
    pub fn spawn(room_id: RoomId, state: Arc<ServerStateInner>) -> RoomHandle {
        let (tx, rx) = mpsc::channel(1024);
        let (snap_tx, snap_rx) = watch::channel(Arc::new(RoomSnapshot::default_with_id(room_id)));

        let actor = Self {
            state: state.clone(),
            room_id,
            snapshot: RoomSnapshot::default_with_id(room_id),
            rx,
            snap_tx,
            member_lists: HashMap::new(),
        };

        tokio::spawn(async move {
            if let Err(e) = actor.run_supervised().await {
                error!(?room_id, "Room actor supervisor failed: {:?}", e);
            }
        });

        RoomHandle {
            room_id,
            tx,
            snapshot: snap_rx,
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

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        if let Err(e) = self.load_initial_state().await {
            if let Error::ApiError(ae) = &e {
                if ae.code == ErrorCode::UnknownRoom {
                    self.snapshot.status = RoomStatus::NotFound;
                    self.publish_snapshot();
                    return Ok(());
                }
            }
            return Err(e);
        }

        self.snapshot.status = RoomStatus::Ready;
        self.publish_snapshot();

        let mut idle_timeout = tokio::time::interval(Duration::from_secs(60 * 15));
        idle_timeout.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            // Message budgeting: Process up to 50 messages before yielding
            let mut processed = 0;
            while processed < 50 {
                match self.rx.try_recv() {
                    Ok(cmd) => {
                        if !self.handle_command_internal(cmd).await? {
                            return Ok(());
                        }
                        processed += 1;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => return Ok(()),
                }
            }

            if processed > 0 {
                tokio::task::yield_now().await;
            }

            tokio::select! {
                _ = idle_timeout.tick() => {
                    // TODO: implement idle check
                }
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            if !self.handle_command_internal(cmd).await? {
                                return Ok(());
                            }
                        }
                        None => return Ok(()),
                    }
                }
            }
        }
    }

    async fn handle_command_internal(&mut self, cmd: RoomCommand) -> Result<bool> {
        match cmd {
            RoomCommand::Sync(msg) => {
                if matches!(msg, MessageSync::RoomDelete { room_id } if room_id == self.room_id) {
                    self.snapshot.status = RoomStatus::NotFound;
                    self.publish_snapshot();
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
                    let mut list = MemberList::new(self.state.clone(), key.clone(), events_tx);
                    let _ = list.initialize(&self.snapshot).await;
                    self.member_lists.insert(key, list);
                }
            }
            RoomCommand::Close => return Ok(false),
        }
        self.publish_snapshot();
        Ok(true)
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

        let mut members = HashMap::new();
        let mut online_count = 0;
        for member in room_members {
            let user = srv.users.get(member.user_id, None).await?;
            if user.presence.status.is_online() {
                online_count += 1;
            }
            members.insert(
                member.user_id,
                Arc::new(CachedRoomMember {
                    member,
                    user: Arc::new(user),
                }),
            );
        }

        let mut roles = HashMap::new();
        for role in roles_data {
            let allow = PermissionBits::from(&role.allow);
            let deny = PermissionBits::from(&role.deny);
            roles.insert(
                role.id,
                Arc::new(CachedRole {
                    inner: role,
                    allow,
                    deny,
                }),
            );
        }

        let mut channels = HashMap::new();
        for channel in channels_data {
            if channel.is_thread() {
                continue;
            }
            let mut overwrites = HashMap::new();
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
                Arc::new(crate::services::cache::room::CachedChannel {
                    inner: channel,
                    overwrites,
                }),
            );
        }

        let mut threads = HashMap::new();
        for thread in active_threads_vec {
            let thread_members_vec = data.thread_member_list_all(thread.id).await?;
            let mut members_map = HashMap::new();
            for member in thread_members_vec {
                members_map.insert(member.user_id, member);
            }
            threads.insert(
                thread.id,
                Arc::new(CachedThread {
                    thread: Arc::new(thread),
                    members: members_map,
                }),
            );
        }

        let mut room = room;
        room.member_count = members.len() as u64;
        room.online_count = online_count;

        self.snapshot = RoomSnapshot {
            room,
            status: RoomStatus::Ready,
            members,
            channels,
            roles,
            threads,
        };

        Ok(())
    }

    fn publish_snapshot(&self) {
        let _ = self.snap_tx.send(Arc::new(self.snapshot.clone()));
    }

    async fn handle_sync(&mut self, event: MessageSync) -> Result<()> {
        match &event {
            MessageSync::RoomUpdate { room } => {
                self.snapshot.room = room.clone();
            }
            MessageSync::ChannelCreate { channel } => {
                if channel.room_id != Some(self.room_id) {
                    return Ok(());
                }
                let mut overwrites = HashMap::new();
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
                    self.snapshot.threads.insert(
                        channel.id,
                        Arc::new(CachedThread {
                            thread: Arc::new(*channel.clone()),
                            members: HashMap::new(),
                        }),
                    );
                } else {
                    self.snapshot.channels.insert(
                        channel.id,
                        Arc::new(crate::services::cache::room::CachedChannel {
                            inner: *channel.clone(),
                            overwrites,
                        }),
                    );
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                if channel.room_id != Some(self.room_id) {
                    return Ok(());
                }
                let mut overwrites = HashMap::new();
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
                        self.snapshot.threads.remove(&channel.id);
                    } else {
                        self.snapshot
                            .threads
                            .entry(channel.id)
                            .and_modify(|t| {
                                let mut updated = t.as_ref().clone();
                                updated.thread = Arc::new(*channel.clone());
                                *t = Arc::new(updated);
                            })
                            .or_insert_with(|| {
                                Arc::new(CachedThread {
                                    thread: Arc::new(*channel.clone()),
                                    members: HashMap::new(),
                                })
                            });
                    }
                } else if channel.is_removed() {
                    self.snapshot.channels.remove(&channel.id);
                } else {
                    self.snapshot.channels.insert(
                        channel.id,
                        Arc::new(crate::services::cache::room::CachedChannel {
                            inner: *channel.clone(),
                            overwrites,
                        }),
                    );
                }
            }
            MessageSync::RoleCreate { role } => {
                if role.room_id != self.room_id {
                    return Ok(());
                }
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                self.snapshot.roles.insert(
                    role.id,
                    Arc::new(CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    }),
                );
            }
            MessageSync::RoleUpdate { role } => {
                if role.room_id != self.room_id {
                    return Ok(());
                }
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                self.snapshot.roles.insert(
                    role.id,
                    Arc::new(CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    }),
                );
            }
            MessageSync::RoleDelete { role_id, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                self.snapshot.roles.remove(role_id);
                for member in self.snapshot.members.values_mut() {
                    let mut m = member.as_ref().clone();
                    m.member.roles.retain(|r| r != role_id);
                    *member = Arc::new(m);
                }
            }
            MessageSync::RoleReorder { roles, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                for item in roles {
                    if let Some(role) = self.snapshot.roles.get_mut(&item.role_id) {
                        let mut r = role.as_ref().clone();
                        r.inner.position = item.position;
                        *role = Arc::new(r);
                    }
                }
            }
            MessageSync::RoomMemberCreate { member, user } => {
                if member.room_id != self.room_id {
                    return Ok(());
                }
                if !self.snapshot.members.contains_key(&member.user_id) {
                    self.snapshot.room.member_count += 1;
                    if user.presence.status.is_online() {
                        self.snapshot.room.online_count += 1;
                    }
                }
                self.snapshot.members.insert(
                    member.user_id,
                    Arc::new(CachedRoomMember {
                        member: member.clone(),
                        user: Arc::new(user.clone()),
                    }),
                );
            }
            MessageSync::RoomMemberUpdate { member, user } => {
                if member.room_id != self.room_id {
                    return Ok(());
                }
                if let Some(old_member) = self.snapshot.members.get(&member.user_id) {
                    let old_online = old_member.user.presence.status.is_online();
                    let new_online = user.presence.status.is_online();
                    if old_online != new_online {
                        if new_online {
                            self.snapshot.room.online_count += 1;
                        } else {
                            self.snapshot.room.online_count =
                                self.snapshot.room.online_count.saturating_sub(1);
                        }
                    }
                }
                self.snapshot.members.insert(
                    member.user_id,
                    Arc::new(CachedRoomMember {
                        member: member.clone(),
                        user: Arc::new(user.clone()),
                    }),
                );
            }
            MessageSync::RoomMemberDelete { user_id, room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                if let Some(member) = self.snapshot.members.remove(user_id) {
                    self.snapshot.room.member_count =
                        self.snapshot.room.member_count.saturating_sub(1);
                    if member.user.presence.status.is_online() {
                        self.snapshot.room.online_count =
                            self.snapshot.room.online_count.saturating_sub(1);
                    }
                }
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                if let Some(member) = self.snapshot.members.get_mut(user_id) {
                    let old_online = member.user.presence.status.is_online();
                    let new_online = presence.status.is_online();
                    if old_online != new_online {
                        if new_online {
                            self.snapshot.room.online_count += 1;
                        } else {
                            self.snapshot.room.online_count =
                                self.snapshot.room.online_count.saturating_sub(1);
                        }
                    }

                    let mut m = member.as_ref().clone();
                    let mut u = m.user.as_ref().clone();
                    u.presence = presence.clone();
                    m.user = Arc::new(u);
                    *member = Arc::new(m);
                }
            }
            MessageSync::UserUpdate { user } => {
                if let Some(member) = self.snapshot.members.get_mut(&user.id) {
                    let old_online = member.user.presence.status.is_online();
                    let new_online = user.presence.status.is_online();
                    if old_online != new_online {
                        if new_online {
                            self.snapshot.room.online_count += 1;
                        } else {
                            self.snapshot.room.online_count =
                                self.snapshot.room.online_count.saturating_sub(1);
                        }
                    }

                    let mut m = member.as_ref().clone();
                    m.user = Arc::new(user.clone());
                    *member = Arc::new(m);
                }
            }
            MessageSync::MessageCreate { message } | MessageSync::MessageUpdate { message } => {
                if message.room_id != Some(self.room_id) {
                    return Ok(());
                }
                // We don't store messages in RoomSnapshot, but we might need to update member list etc.
            }
            MessageSync::MessageDelete { .. }
            | MessageSync::MessageDeleteBulk { .. }
            | MessageSync::MessageRemove { .. }
            | MessageSync::MessageRestore { .. } => {
                // Messages are not in RoomSnapshot
            }
            MessageSync::ThreadMemberUpsert {
                thread_id,
                added,
                removed,
                ..
            } => {
                if let Some(thread) = self.snapshot.threads.get_mut(thread_id) {
                    let mut t = thread.as_ref().clone();
                    for member in added {
                        t.members.insert(member.user_id, member.clone());
                    }
                    for user_id in removed {
                        t.members.remove(user_id);
                    }
                    *thread = Arc::new(t);
                }
            }
            _ => {}
        }

        // Notify member lists of the event
        for list in self.member_lists.values_mut() {
            list.handle_sync(event.clone(), &self.snapshot).await;
        }

        Ok(())
    }
}
