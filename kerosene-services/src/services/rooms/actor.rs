use common::v1::types::error::ErrorCode;
use common::v1::types::{MessageSync, RoomId, User, UserId};
use im::HashMap as ImMap;
use kameo::prelude::{Actor, ActorRef, Spawn, WeakActorRef};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;
use tokio::time::Duration;
use tracing::Instrument;

use super::{
    CachedPermissionOverwrite, CachedRole, CachedRoomMember, CachedThread, LoadedRoom, RoomSnapshot,
};
use crate::prelude::*;
use crate::services::member_lists::actor::MemberList;
use crate::services::member_lists::util::MemberListKey;
use crate::services::rooms::types::RoomMembers;
use crate::types::PermissionBits;
use crate::{Error, Result};

// PERF: how many `Arc`s are too many?

/// The internal state of a room actor.
pub struct RoomActor {
    state: Globals,
    room_id: RoomId,
    snapshot: Arc<RoomSnapshot>,
    snapshot_tx: watch::Sender<Arc<RoomSnapshot>>,
    member_lists: HashMap<MemberListKey, MemberList>,
    last_active: Instant,
    span: tracing::Span,
}

impl Actor for RoomActor {
    type Args = (RoomId, Globals, watch::Sender<Arc<RoomSnapshot>>);
    type Error = Error;

    async fn on_start(
        (room_id, state, snapshot_tx): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> std::result::Result<Self, Self::Error> {
        let span = tracing::info_span!("room actor");

        let snapshot = Arc::new(RoomSnapshot::loading());
        let mut actor = Self {
            state,
            room_id,
            snapshot,
            snapshot_tx,
            member_lists: HashMap::new(),
            last_active: Instant::now(),
            span: span.clone(),
        };

        let cleanup_ref = actor_ref.clone();
        let cleanup_span = span.clone();
        tokio::spawn(
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    if cleanup_ref.tell(CleanupIdleLists).await.is_err() {
                        break;
                    }
                }
            }
            .instrument(cleanup_span),
        );

        async {
            // Load initial state
            if let Err(e) = actor.load_initial_state().await {
                if let Error::ApiError(ae) = &e {
                    if ae.code == ErrorCode::UnknownRoom {
                        actor.snapshot = Arc::new(RoomSnapshot::not_found());
                        let _ = actor.snapshot_tx.send(Arc::clone(&actor.snapshot));
                        return Ok(actor);
                    }
                }
                return Err(e);
            }

            let _ = actor.snapshot_tx.send(Arc::clone(&actor.snapshot));
            Ok(actor)
        }
        .instrument(span)
        .await
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: kameo::prelude::ActorStopReason,
    ) -> std::result::Result<(), Self::Error> {
        // cleanup: unregister all members from the cache
        if let RoomSnapshot::Available(data) = self.snapshot.as_ref() {
            let srv = self.state.services();
            if let RoomMembers::Loaded { members } = &data.members {
                for user_id in members.keys() {
                    srv.rooms.member_unregister(*user_id, self.room_id);
                }
            }
        }
        Ok(())
    }
}

impl RoomActor {
    pub fn spawn_room(room_id: RoomId, state: Globals) -> RoomHandle {
        let (snapshot_tx, snapshot_rx) = watch::channel(Arc::new(RoomSnapshot::loading()));
        let actor_ref = RoomActor::spawn((room_id, state, snapshot_tx));
        RoomHandle {
            room_id,
            actor_ref,
            snapshot_rx,
        }
    }

    async fn load_initial_state(&mut self) -> Result<()> {
        let mut data = self.state.begin_read().await?;
        let srv = self.state.services();

        let root_span = tracing::info_span!("room_load", room_id = ?self.room_id);

        // PERF: fetch these all in parallel
        let room = data
            .room_get(self.room_id)
            .instrument(tracing::info_span!("room_load.query.room"))
            .await?;
        let room_members = data
            .room_member_list_all(self.room_id)
            .instrument(tracing::info_span!("room_load.query.members"))
            .await?;
        let roles_data = data
            .role_list(self.room_id)
            .instrument(tracing::info_span!("room_load.query.roles"))
            .await?;
        let channels_data = data
            .channel_list(self.room_id)
            .instrument(tracing::info_span!("room_load.query.channels"))
            .await?;
        let active_threads_vec = data
            .thread_all_active_room(self.room_id)
            .instrument(tracing::info_span!("room_load.query.threads"))
            .await?;

        root_span.record("room_members_count", room_members.len());
        root_span.record("roles_count", roles_data.len());
        root_span.record("channels_count", channels_data.len());
        root_span.record("threads_count", active_threads_vec.len());

        let user_ids: Vec<_> = room_members.iter().map(|m| m.user_id).collect();

        // PERF: fetch thread members in parallel
        let thread_ids: Vec<_> = active_threads_vec.iter().map(|t| t.id).collect();

        let users = srv.users.get_many(&user_ids).await?;
        let mut all_thread_members = Vec::with_capacity(thread_ids.len());
        for tid in thread_ids {
            let members = data
                .thread_member_list_all(tid)
                .instrument(tracing::info_span!("room_load.query.thread_members", thread_id = ?tid))
                .await?;
            all_thread_members.push(members);
        }

        let users_map: HashMap<UserId, Arc<User>> =
            users.into_iter().map(|u| (u.id, Arc::new(u))).collect();

        let mut members = ImMap::new();
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
                srv.rooms.member_register(user_id, self.room_id);
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

        self.snapshot = Arc::new(RoomSnapshot::Available(Arc::new(LoadedRoom {
            room: Arc::new(room),
            members: RoomMembers::Loaded { members },
            channels,
            roles,
            threads: Some(threads),
        })));

        Ok(())
    }

    /// Load members for a room that is in WithoutMembers state.
    async fn load_members(&mut self) -> Result<()> {
        let mut data = self.state.begin_read().await?;
        let srv = self.state.services();

        let current_data = match self.snapshot.as_ref() {
            RoomSnapshot::Available(data) => data.as_ref().clone(),
            _ => return Ok(()),
        };

        let room_members = data.room_member_list_all(self.room_id).await?;

        let user_ids: Vec<_> = room_members.iter().map(|m| m.user_id).collect();
        let users = srv.users.get_many(&user_ids).await?;
        let users_map: HashMap<UserId, Arc<User>> =
            users.into_iter().map(|u| (u.id, Arc::new(u))).collect();

        let mut members = ImMap::new();
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
                srv.rooms.member_register(user_id, self.room_id);
            }
        }

        let new_data = Arc::new(LoadedRoom {
            members: RoomMembers::Loaded { members },
            ..current_data
        });

        self.snapshot = Arc::new(RoomSnapshot::Available(new_data));

        Ok(())
    }

    async fn handle_sync(&mut self, event: MessageSync) -> Result<()> {
        match self.snapshot.as_ref() {
            RoomSnapshot::Available(loaded) => {
                // PERF: return early if event isnt applicable
                let new = loaded.apply(&event);
                self.snapshot = Arc::new(RoomSnapshot::Available(Arc::new(new)));
            }
            _ => return Ok(()),
        };

        match &event {
            MessageSync::RoomDelete { room_id } if self.room_id == *room_id => {
                self.snapshot = Arc::new(RoomSnapshot::deleted());
            }
            MessageSync::RoomMemberCreate { member, .. } if self.room_id == member.room_id => {
                self.state
                    .services()
                    .rooms
                    .member_register(member.user_id, self.room_id);
            }
            MessageSync::RoomMemberDelete { room_id, user_id } if self.room_id == *room_id => {
                self.state
                    .services()
                    .rooms
                    .member_unregister(*user_id, *room_id);
            }
            _ => {}
        }

        for list in self.member_lists.values_mut() {
            list.handle_sync(event.clone(), Arc::clone(&self.snapshot))
                .await;
        }

        let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));

        Ok(())
    }
}

// TODO: use tracing::instrument macro
// something like #[tracing::instrument(parent = &self.span, skip(self), name = "get_snapshot")]

#[kameo::messages]
impl RoomActor {
    #[message]
    pub async fn get_snapshot(&mut self) -> Result<Arc<RoomSnapshot>> {
        let span = tracing::info_span!(parent: &self.span, "GetSnapshot");
        async {
            self.last_active = Instant::now();
            Ok(Arc::clone(&self.snapshot))
        }
        .instrument(span)
        .await
    }

    #[message]
    pub async fn ensure_members(&mut self) -> Result<()> {
        let span = tracing::info_span!(parent: &self.span, "EnsureMembers");
        async {
            self.last_active = Instant::now();
            if self
                .snapshot
                .get_data()
                .map_or(false, |d| matches!(d.members, RoomMembers::Loading))
            {
                self.load_members().await?;
                let _ = self.snapshot_tx.send(Arc::clone(&self.snapshot));
            }
            Ok(())
        }
        .instrument(span)
        .await
    }

    #[message]
    pub async fn sync_message(&mut self, sync: MessageSync) -> Result<()> {
        let span = tracing::info_span!(parent: &self.span, "SyncMessage");
        async {
            self.last_active = Instant::now();
            self.handle_sync(sync).await
        }
        .instrument(span)
        .await
    }

    #[message]
    pub async fn member_list_command_msg(
        &mut self,
        key: crate::services::member_lists::util::MemberListKey,
        cmd: crate::services::member_lists::actor::MemberListCommand,
    ) -> Result<Option<MessageSync>> {
        let span = tracing::info_span!(parent: &self.span, "MemberListCommandMsg");
        async {
            self.last_active = Instant::now();
            if let Some(list) = self.member_lists.get_mut(&key) {
                Ok(list.handle_command(cmd, &self.snapshot).await)
            } else {
                Ok(None)
            }
        }
        .instrument(span)
        .await
    }

    #[message]
    pub async fn member_list_subscribe_msg(
        &mut self,
        key: crate::services::member_lists::util::MemberListKey,
        events_tx: tokio::sync::broadcast::Sender<
            crate::services::member_lists::actor::MemberListEvent,
        >,
    ) -> Result<()> {
        self.last_active = Instant::now();
        if !self.member_lists.contains_key(&key) {
            if self
                .snapshot
                .get_data()
                .map_or(false, |d| matches!(d.members, RoomMembers::Loading))
            {
                self.load_members().await?;
            }
            let mut list = MemberList::new(self.state.clone(), key.clone(), events_tx);
            let _ = list.initialize(Arc::clone(&self.snapshot)).await;
            self.member_lists.insert(key, list);
        }
        Ok(())
    }

    #[message]
    pub fn cleanup_idle_lists(&mut self) {
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

// pub enum RoomEvent {
//     /// a sync event happened in this room
//     Sync(MessageSync),

//     /// the room's snapshot changed
//     Update(RoomSnapshot),

//     /// room was unloaded
//     ///
//     /// this is not emitted if the room is being reloaded. instead, the room snapshot state will become `Loading`
//     Unload,
// }

/// a handle for interacting with a room actor
#[derive(Clone)]
pub struct RoomHandle {
    pub room_id: RoomId,
    pub actor_ref: ActorRef<RoomActor>,
    pub snapshot_rx: watch::Receiver<Arc<RoomSnapshot>>,
}

impl RoomHandle {
    pub fn room_id(&self) -> RoomId {
        self.room_id
    }

    // /// wait until the room has successfully loaded
    // ///
    // /// - `with_members` will wait until all room members are loaded
    // /// - `fail_if_unavailable` returns an error if the room is or becomes unavailable
    // pub async fn ready(
    //     &mut self,
    //     with_members: bool,
    //     fail_if_unavailable: bool,
    // ) -> Result<Arc<RoomData>> {
    //     let s = self
    //         .snapshot
    //         .wait_for(|s| match &s.state {
    //             RoomSnapshotState::Loading => false,
    //             // RoomSnapshotState::Loaded(data) => !with_members || data.members_loaded,
    //             RoomSnapshotState::Loaded(data) => todo!(),
    //             RoomSnapshotState::Unavailable(r) => r.is_fatal() || fail_if_unavailable,
    //         })
    //         .await
    //         .expect("todo better error handling");
    //     let data = match &s.state {
    //         RoomSnapshotState::Loaded(data) => Arc::clone(data),
    //         RoomSnapshotState::Unavailable(_) => todo!("return err"),
    //         _ => unreachable!(),
    //     };
    //     Ok(data)
    // }

    // /// get the current room snapshot
    // pub fn snapshot(&self) -> Arc<RoomSnapshot> {
    //     Arc::clone(&self.snapshot.borrow())
    // }

    // /// get the current room data
    // pub fn data(&self) -> Result<Arc<RoomData>> {
    //     match &self.snapshot.borrow().state {
    //         RoomSnapshotState::Loading => Err(Error::BadStatic("room is still loading")),
    //         RoomSnapshotState::Loaded(_) => todo!(),
    //         RoomSnapshotState::Unavailable(_) => Err(Error::BadStatic("room is unavailable")),
    //     }
    // }

    // pub fn subscribe(&self) -> mpsc::Receiver<Arc<RoomEvent>> {}

    // pub fn reload(&self) {
    //     todo!()
    // }

    // pub fn unload(&self) {
    //     todo!()
    // }

    // /// create a subscription to a member list
    // pub fn member_list(&self, conn_id: ConnectionId) -> MemberList {
    //     todo!()
    // }

    // pub(super) fn handle_sync(&self, sync: MessageSync) {
    //     todo!()
    // }
}
