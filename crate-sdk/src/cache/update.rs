use std::sync::Arc;

use arc_swap::ArcSwap;
use common::v1::types::{MessagePayload, MessageSync};

use crate::cache::{Cache, CacheCurrent, CachedChannel, CachedRoom, CachedUser, PermSet};

impl Cache {
    /// update this cache based off of a sync event
    pub fn update(&self, payload: &MessagePayload) {
        match payload {
            MessagePayload::Sync { data, .. } => match &**data {
                MessageSync::Ambient {
                    rooms,
                    roles,
                    channels,
                    threads,
                    room_members,
                    ..
                } => {
                    for room in rooms {
                        self.inner
                            .rooms
                            .insert(room.id, Arc::new(CachedRoom::new(room.clone())));
                    }

                    for role in roles {
                        if let Some(room) = self.inner.rooms.get(&role.room_id) {
                            room.roles.insert(role.id, role.clone());
                            room.perm_roles
                                .insert(role.id, (PermSet::from(role), role.position));
                        }
                    }

                    for channel in channels {
                        let channel_id = channel.id;
                        let cached = Arc::new(CachedChannel::new(channel.clone()));
                        self.inner.channels.insert(channel_id, cached.clone());
                        if let Some(room_id) = channel.room_id {
                            if let Some(room) = self.inner.rooms.get(&room_id) {
                                room.channels.insert(channel_id, cached);
                            }
                        }
                    }

                    for channel in threads {
                        let channel_id = channel.id;
                        let cached = Arc::new(CachedChannel::new(channel.clone()));
                        self.inner.channels.insert(channel_id, cached.clone());
                        if let Some(room_id) = channel.room_id {
                            if let Some(room) = self.inner.rooms.get(&room_id) {
                                room.threads.insert(channel_id, cached);
                            }
                        }
                    }

                    for member in room_members {
                        if let Some(room) = self.inner.rooms.get(&member.room_id) {
                            room.members.insert(member.user_id, member.clone());
                        }
                    }
                }
                MessageSync::RoomCreate { room } => {
                    self.inner
                        .rooms
                        .insert(room.id, Arc::new(CachedRoom::new(room.clone())));
                }
                MessageSync::RoomUpdate { room } => {
                    if let Some(cached_room) = self.inner.rooms.get(&room.id) {
                        cached_room.inner.store(Arc::new(room.clone()));
                    }
                }
                MessageSync::RoleCreate { role } => {
                    if let Some(cached_room) = self.inner.rooms.get(&role.room_id) {
                        cached_room.roles.insert(role.id, role.clone());
                        cached_room
                            .perm_roles
                            .insert(role.id, (PermSet::from(role), role.position));
                    }
                }
                MessageSync::RoleUpdate { role } => {
                    if let Some(cached_room) = self.inner.rooms.get(&role.room_id) {
                        cached_room.roles.insert(role.id, role.clone());
                        cached_room
                            .perm_roles
                            .insert(role.id, (PermSet::from(role), role.position));
                    }
                }
                MessageSync::RoleDelete { room_id, role_id } => {
                    if let Some(cached_room) = self.inner.rooms.get(&room_id) {
                        cached_room.roles.remove(&role_id);
                        cached_room.perm_roles.remove(&role_id);
                    }
                }
                MessageSync::ChannelCreate { channel } => {
                    let channel_id = channel.id;
                    let chan = Arc::new(CachedChannel::new(*channel.clone()));
                    self.inner.channels.insert(channel_id, chan.clone());
                    if let Some(room_id) = chan.inner.load().room_id {
                        if let Some(cached_room) = self.inner.rooms.get(&room_id) {
                            cached_room.channels.insert(channel_id, chan);
                        }
                    }
                }
                MessageSync::ChannelUpdate { channel } => {
                    if let Some(cached_channel) = self.inner.channels.get(&channel.id) {
                        cached_channel.inner.store(Arc::new(*channel.clone()));
                    }
                }
                MessageSync::RoomMemberCreate { member, .. } => {
                    if let Some(cached_room) = self.inner.rooms.get(&member.room_id) {
                        cached_room.members.insert(member.user_id, member.clone());
                    }
                }
                MessageSync::RoomMemberUpdate { member, .. } => {
                    if let Some(cached_room) = self.inner.rooms.get(&member.room_id) {
                        cached_room.members.insert(member.user_id, member.clone());
                    }
                }
                MessageSync::RoomMemberDelete { room_id, user_id } => {
                    if let Some(cached_room) = self.inner.rooms.get(&room_id) {
                        cached_room.members.remove(&user_id);
                    }
                }
                MessageSync::UserUpdate { user } => {
                    if let Some(cached_user) = self.inner.users.get(&user.id) {
                        cached_user.inner.store(Arc::new(user.clone()));
                    }
                }
                MessageSync::PresenceUpdate { user_id, presence } => {
                    if let Some(cached_user) = self.inner.users.get(&user_id) {
                        cached_user.presence.store(Arc::new(presence.clone()));
                    }
                }
                MessageSync::VoiceState {
                    user_id,
                    state,
                    old_state,
                } => match (old_state, state) {
                    (Some(old_state), Some(state)) => {
                        if old_state.channel_id != state.channel_id {
                            // switched channels
                            if let Some(cached_channel) =
                                self.inner.channels.get(&old_state.channel_id)
                            {
                                cached_channel.voice_states.remove(user_id);
                            }
                        }

                        // voice state update
                        if let Some(cached_channel) = self.inner.channels.get(&state.channel_id) {
                            cached_channel
                                .voice_states
                                .insert(*user_id, Arc::new(state.clone()));
                        }
                    }
                    (Some(old_state), None) => {
                        // disconnected
                        if let Some(cached_channel) = self.inner.channels.get(&old_state.channel_id)
                        {
                            cached_channel.voice_states.remove(user_id);
                        }
                    }
                    (None, Some(state)) => {
                        // newly connected
                        if let Some(cached_channel) = self.inner.channels.get(&state.channel_id) {
                            cached_channel
                                .voice_states
                                .insert(*user_id, Arc::new(state.clone()));
                        }
                    }
                    (None, None) => {
                        // two options
                        // 1. user joined a channel we can't see (None -> state)
                        // 2. user left a channel we can't see (state -> None)
                        // just clear every state to be safe
                        for cached_channel in self.inner.channels.iter() {
                            cached_channel.voice_states.remove(user_id);
                        }
                    }
                },
                // TODO: handle RoleReorder
                // TODO: handle ChannelReorder
                // TODO: handle messages and reactions
                _ => {}
            },
            MessagePayload::Ready { user, session, .. } => {
                if let Some(user) = user {
                    self.inner.users.insert(
                        user.id,
                        Arc::new(CachedUser {
                            inner: ArcSwap::from_pointee(*user.clone()),
                            presence: ArcSwap::from_pointee(user.presence.clone()),
                        }),
                    );
                }

                self.inner.current.store(Arc::new(CacheCurrent {
                    session_id: Some(session.id),
                    user_id: user.as_ref().map(|u| u.id),
                }));
            }
            _ => {}
        }
    }
}
