use im::HashMap as ImMap;
use kameo::prelude::{Actor, ActorRef, Context, Message, Spawn, WeakActorRef};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;
use tracing::Instrument;

use common::v1::types::{MessageSync, RoomId, User, UserId};
use tokio::sync::watch;

use super::{
    CachedPermissionOverwrite, CachedRole, CachedRoomMember, CachedThread, CleanupIdleLists,
    EnsureMembers, GetSnapshot, MemberListCommandMsg, MemberListSubscribeMsg, RoomData, RoomHandle,
    RoomSnapshot, SyncMessage,
};
use crate::services::member_lists::actor::MemberList;
use crate::services::member_lists::util::MemberListKey;
use crate::types::PermissionBits;
use crate::{Error, Result, ServerStateInner};

/// The internal state of a room actor.
pub struct RoomActor {
    state: Arc<ServerStateInner>,
    room_id: RoomId,
    snapshot: Arc<RoomSnapshot>,
    snapshot_tx: watch::Sender<Arc<RoomSnapshot>>,
    member_lists: HashMap<MemberListKey, MemberList>,
    last_active: Instant,
}

impl Actor for RoomActor {
    type Args = (
        RoomId,
        Arc<ServerStateInner>,
        watch::Sender<Arc<RoomSnapshot>>,
    );
    type Error = Error;

    async fn on_start(
        (room_id, state, snapshot_tx): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> std::result::Result<Self, Self::Error> {
        let snapshot = Arc::new(RoomSnapshot::Loading);

        let mut actor = Self {
            state: state.clone(),
            room_id,
            snapshot,
            snapshot_tx,
            member_lists: HashMap::new(),
            last_active: Instant::now(),
        };

        // Spawn background task to clean up idle member lists
        let cleanup_ref = actor_ref.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                if cleanup_ref.tell(CleanupIdleLists).await.is_err() {
                    break;
                }
            }
        });

        // Load initial state
        if let Err(e) = actor.load_initial_state().await {
            if let Error::ApiError(ae) = &e {
                if ae.code == common::v1::types::error::ErrorCode::UnknownRoom {
                    actor.snapshot = Arc::new(RoomSnapshot::NotFound);
                    let _ = actor.snapshot_tx.send(Arc::clone(&actor.snapshot));
                    return Ok(actor);
                }
            }
            return Err(e);
        }

        let _ = actor.snapshot_tx.send(Arc::clone(&actor.snapshot));
        Ok(actor)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: kameo::prelude::ActorStopReason,
    ) -> std::result::Result<(), Self::Error> {
        // Cleanup: unregister all members from the cache
        if let RoomSnapshot::Ready(data) = self.snapshot.as_ref() {
            let rooms = self.state.services().rooms.clone();
            for user_id in data.members.keys() {
                rooms.member_unregister(*user_id, self.room_id);
            }
        }
        Ok(())
    }
}

impl RoomActor {
    pub fn spawn_room(room_id: RoomId, state: Arc<ServerStateInner>) -> RoomHandle {
        let (snapshot_tx, snapshot_rx) = watch::channel(Arc::new(RoomSnapshot::Loading));

        let actor_ref = Spawn::spawn((room_id, state.clone(), snapshot_tx));

        RoomHandle {
            room_id,
            actor_ref,
            snapshot_rx,
        }
    }

    async fn load_initial_state(&mut self) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let root_span = tracing::info_span!("room_load", room_id = ?self.room_id);

        let (room, room_members, roles_data, channels_data, active_threads_vec) = async {
            tokio::try_join!(
                data.room_get(self.room_id)
                    .instrument(tracing::info_span!("room_load.query.room")),
                data.room_member_list_all(self.room_id)
                    .instrument(tracing::info_span!("room_load.query.members")),
                async {
                    data.role_list(
                        self.room_id,
                        crate::types::PaginationQuery {
                            limit: Some(1024),
                            ..Default::default()
                        },
                    )
                    .instrument(tracing::info_span!("room_load.query.roles"))
                    .await
                    .map(|r| r.items)
                },
                data.channel_list(self.room_id)
                    .instrument(tracing::info_span!("room_load.query.channels")),
                data.thread_all_active_room(self.room_id)
                    .instrument(tracing::info_span!("room_load.query.threads")),
            )
        }
        .instrument(root_span.clone())
        .await?;

        root_span.record("room_members_count", room_members.len());
        root_span.record("roles_count", roles_data.len());
        root_span.record("channels_count", channels_data.len());
        root_span.record("threads_count", active_threads_vec.len());

        let user_ids: Vec<_> = room_members.iter().map(|m| m.user_id).collect();
        let thread_member_futs = active_threads_vec.iter().map(|t| {
            data.thread_member_list_all(t.id).instrument(
                tracing::info_span!("room_load.query.thread_members", thread_id = ?t.id),
            )
        });

        let (users, all_thread_members) = tokio::try_join!(
            srv.users.get_many(&user_ids),
            async { futures::future::try_join_all(thread_member_futs).await }
                .instrument(tracing::info_span!("room_load.query.thread_members_all")),
        )?;

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

    /// Load members for a room that is in WithoutMembers state.
    async fn load_members(&mut self) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let current_data = match self.snapshot.as_ref() {
            RoomSnapshot::WithoutMembers(data) | RoomSnapshot::Ready(data) => data.as_ref().clone(),
            _ => return Ok(()),
        };

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

        let new_data = Arc::new(RoomData {
            members,
            ..current_data
        });

        self.snapshot = Arc::new(RoomSnapshot::Ready(new_data));

        Ok(())
    }

    async fn handle_sync(&mut self, event: MessageSync) -> Result<()> {
        let mut snapshot_data = match self.snapshot.as_ref() {
            RoomSnapshot::Ready(data) | RoomSnapshot::WithoutMembers(data) => data.as_ref().clone(),
            _ => return Ok(()),
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
                }

                self.state
                    .services()
                    .rooms
                    .member_unregister(*user_id, self.room_id);
            }
            MessageSync::RoomDelete { room_id } => {
                if *room_id != self.room_id {
                    return Ok(());
                }
                self.snapshot = Arc::new(RoomSnapshot::NotFound);
            }
            MessageSync::ThreadMemberUpsert {
                room_id: msg_room_id,
                thread_id,
                added,
                removed,
            } => {
                if *msg_room_id != Some(self.room_id) {
                    return Ok(());
                }

                if let RoomSnapshot::Ready(_) = self.snapshot.as_ref() {
                    for member in added {
                        // First check if thread exists
                        let thread_exists = snapshot_data.threads.contains_key(thread_id);

                        if thread_exists {
                            snapshot_data.threads.entry(*thread_id).and_modify(|t| {
                                t.members.insert(member.user_id, member.clone());
                            });
                        } else {
                            // Thread doesn't exist, try to create it from channels
                            if let Some(cached_channel) = snapshot_data.channels.get(thread_id) {
                                snapshot_data.threads.insert(
                                    *thread_id,
                                    CachedThread {
                                        thread: cached_channel.inner.clone(),
                                        members: ImMap::from_iter([(
                                            member.user_id,
                                            member.clone(),
                                        )]),
                                    },
                                );
                            }
                            // If channel doesn't exist either, skip adding the member
                        }
                    }

                    for user_id in removed {
                        if let Some(thread) = snapshot_data.threads.get_mut(thread_id) {
                            thread.members.remove(user_id);
                        }
                    }
                }
            }
            _ => {}
        }

        self.snapshot = Arc::new(RoomSnapshot::Ready(Arc::new(snapshot_data)));
        let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));

        Ok(())
    }
}

impl Message<GetSnapshot> for RoomActor {
    type Reply = Result<Arc<RoomSnapshot>>;

    async fn handle(
        &mut self,
        _msg: GetSnapshot,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        Ok(Arc::clone(&self.snapshot))
    }
}

impl Message<EnsureMembers> for RoomActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        _msg: EnsureMembers,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        if self.snapshot.is_without_members() {
            self.load_members().await?;
            let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));
        }
        Ok(())
    }
}

impl Message<SyncMessage> for RoomActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: SyncMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        self.handle_sync(msg.sync).await
    }
}

impl Message<MemberListCommandMsg> for RoomActor {
    type Reply = Result<Option<MessageSync>>;

    async fn handle(
        &mut self,
        msg: MemberListCommandMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        if let Some(list) = self.member_lists.get_mut(&msg.key) {
            Ok(list.handle_command(msg.cmd, &self.snapshot).await)
        } else {
            Ok(None)
        }
    }
}

impl Message<MemberListSubscribeMsg> for RoomActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: MemberListSubscribeMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        if !self.member_lists.contains_key(&msg.key) {
            if self.snapshot.is_without_members() {
                self.load_members().await?;
            }
            let mut list = MemberList::new(self.state.clone(), msg.key.clone(), msg.events_tx);
            let _ = list.initialize(Arc::clone(&self.snapshot)).await;
            self.member_lists.insert(msg.key, list);
        }
        Ok(())
    }
}

impl Message<CleanupIdleLists> for RoomActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: CleanupIdleLists,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.member_lists.retain(|key, list| {
            if list.is_idle() {
                tracing::trace!(room_id = ?self.room_id, ?key, "Removing idle member list");
                false
            } else {
                true
            }
        });
    }
}
