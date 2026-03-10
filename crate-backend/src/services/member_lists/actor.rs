use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use common::v1::types::{
    MemberListGroup, MemberListOp, MessageSync, Permission, RoleId, RoomMember, User, UserId,
};
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

use crate::services::cache::permissions::PermissionsCalculator;
use crate::services::cache::room::RoomSnapshot;
use crate::{
    services::member_lists::util::{MemberGroupInfo, MemberKey, MemberListKey},
    Result, ServerStateInner,
};

/// member list state machine, now owned by RoomActor
pub struct MemberList {
    pub(super) s: Arc<ServerStateInner>,
    pub(super) key: MemberListKey,

    /// ordered map of members for range queries and position tracking
    pub(super) members: BTreeMap<MemberKey, UserId>,

    /// reverse lookup: UserId -> MemberKey
    pub(super) user_to_key: HashMap<UserId, MemberKey>,

    /// group summaries (id and count)
    pub(super) groups: Vec<MemberListGroup>,

    pub(super) events_tx: broadcast::Sender<MemberListEvent>,
}

pub enum MemberListCommand {
    GetInitialRanges {
        ranges: Vec<(u64, u64)>,
        conn_id: Uuid,
        callback: oneshot::Sender<MessageSync>,
    },
}

#[derive(Debug, Clone)]
pub enum MemberListEvent {
    Broadcast(MessageSync),
    Unicast(Uuid, MessageSync),
}

impl MemberList {
    pub fn new(
        s: Arc<ServerStateInner>,
        key: MemberListKey,
        events_tx: broadcast::Sender<MemberListEvent>,
    ) -> Self {
        Self {
            s,
            key,
            members: BTreeMap::new(),
            user_to_key: HashMap::new(),
            groups: Vec::new(),
            events_tx,
        }
    }

    pub async fn initialize(&mut self, snapshot: &RoomSnapshot) -> Result<()> {
        let room_id = match &self.key {
            MemberListKey::Room(id) => Some(*id),
            MemberListKey::RoomChannel(id, _) => Some(*id),
            MemberListKey::RoomThread(id, _, _) => Some(*id),
            MemberListKey::Dm(_) => None,
        };

        if let Some(_room_id) = room_id {
            let thread_members = if let MemberListKey::RoomThread(_, _, channel_id) = &self.key {
                let list = self.s.data().thread_member_list_all(*channel_id).await?;
                Some(
                    list.into_iter()
                        .map(|m| (m.user_id, m))
                        .collect::<HashMap<_, _>>(),
                )
            } else {
                None
            };

            let user_ids: Vec<_> = if let Some(ref tm) = thread_members {
                tm.keys().copied().collect()
            } else {
                snapshot.members.keys().copied().collect()
            };

            // fetch all users in the room to get presences and names
            let users = self.s.services().users.get_many(&user_ids).await?;
            let users_map: HashMap<_, _> = users.into_iter().map(|u| (u.id, u)).collect();

            let perms_calc = PermissionsCalculator {
                room_id: snapshot.room.id,
                owner_id: snapshot.room.owner_id,
                public: snapshot.room.public,
                room: Arc::new(snapshot.clone()),
            };

            self.members.clear();
            self.user_to_key.clear();

            for user_id in user_ids {
                if let Some(_user) = users_map.get(&user_id) {
                    if let Some(cached_member) = snapshot.members.get(&user_id) {
                        let is_thread_member = thread_members
                            .as_ref()
                            .map_or(true, |tm| tm.contains_key(&user_id));
                        if is_thread_member
                            && self.should_include(
                                &user_id,
                                &cached_member.member,
                                &perms_calc,
                                snapshot,
                            )
                        {
                            let key = self.calculate_key(
                                &user_id,
                                &cached_member.member,
                                &users_map,
                                snapshot,
                            );
                            self.members.insert(key.clone(), user_id);
                            self.user_to_key.insert(user_id, key);
                        }
                    }
                }
            }
        }

        self.recalculate_groups();
        Ok(())
    }

    fn should_include(
        &self,
        user_id: &UserId,
        member: &RoomMember,
        perms_calc: &PermissionsCalculator,
        snapshot: &RoomSnapshot,
    ) -> bool {
        match &self.key {
            MemberListKey::Room(_) => true,
            MemberListKey::RoomChannel(_, visibility) => {
                if Some(*user_id) == perms_calc.owner_id {
                    return true;
                }
                let (has_admin, has_view) = self.calc_view_base(member, perms_calc);
                if has_admin {
                    return true;
                }
                visibility.visible_to(member, has_view)
            }
            MemberListKey::RoomThread(_, _, channel_id) => {
                // for threads, usually only thread members are shown
                snapshot
                    .threads
                    .get(channel_id)
                    .map_or(false, |t| t.members.contains_key(user_id))
            }
            MemberListKey::Dm(_) => true, // DM lists always include recipients
        }
    }

    fn calc_view_base(
        &self,
        member: &RoomMember,
        perms_calc: &PermissionsCalculator,
    ) -> (bool, bool) {
        let mut has_admin = false;
        let mut has_view_allow = false;
        let mut has_view_deny = false;

        let everyone_role_id = perms_calc.room_id.into_inner().into();

        for role in perms_calc.room.roles.values() {
            if role.inner.id == everyone_role_id || member.roles.contains(&role.inner.id) {
                if role.allow.has(Permission::Admin) {
                    has_admin = true;
                    break;
                }
                if role.allow.has(Permission::ViewChannel) {
                    has_view_allow = true;
                }
                if role.deny.has(Permission::ViewChannel) {
                    has_view_deny = true;
                }
            }
        }

        let has_view = if has_admin {
            true
        } else {
            has_view_allow && !has_view_deny
        };

        (has_admin, has_view)
    }

    fn calculate_key(
        &self,
        user_id: &UserId,
        member: &RoomMember,
        users: &HashMap<UserId, User>,
        snapshot: &RoomSnapshot,
    ) -> MemberKey {
        let user = users.get(user_id).unwrap();
        let is_online = user.presence.is_online();

        let group = if is_online {
            // find highest hoisted role
            let mut best_role: Option<(RoleId, u64)> = None;
            for role_id in &member.roles {
                if let Some(role) = snapshot.roles.get(role_id) {
                    if role.inner.hoist {
                        if best_role.is_none() || role.inner.position < best_role.unwrap().1 {
                            best_role = Some((*role_id, role.inner.position as u64));
                        }
                    }
                }
            }

            if let Some((role_id, role_position)) = best_role {
                MemberGroupInfo::Hoisted {
                    role_position,
                    role_id,
                }
            } else {
                MemberGroupInfo::Online
            }
        } else {
            MemberGroupInfo::Offline
        };

        let name = member
            .override_name
            .as_deref()
            .unwrap_or(user.name.as_str());

        MemberKey {
            group,
            name: Arc::from(name),
            user_id: *user_id,
        }
    }

    pub fn recalculate_groups(&mut self) {
        let mut new_groups = Vec::new();
        if self.members.is_empty() {
            self.groups = new_groups;
            return;
        }

        let mut current_group: Option<MemberGroupInfo> = None;
        let mut count = 0;

        for key in self.members.keys() {
            if Some(key.group) != current_group {
                if let Some(info) = current_group {
                    new_groups.push(MemberListGroup {
                        id: info.into(),
                        count,
                    });
                }
                current_group = Some(key.group);
                count = 1;
            } else {
                count += 1;
            }
        }

        if let Some(info) = current_group {
            new_groups.push(MemberListGroup {
                id: info.into(),
                count,
            });
        }

        self.groups = new_groups;
    }

    pub async fn handle_command(&mut self, cmd: MemberListCommand, snapshot: &RoomSnapshot) {
        match cmd {
            MemberListCommand::GetInitialRanges {
                ranges,
                conn_id: _,
                callback,
            } => {
                let ops = self.get_initial_ranges(&ranges, snapshot).await;
                let _ = callback.send(MessageSync::MemberListSync {
                    user_id: UserId::new(), // dummy

                    room_id: match &self.key {
                        MemberListKey::Room(id) => Some(*id),
                        MemberListKey::RoomChannel(id, _) => Some(*id),
                        MemberListKey::RoomThread(id, _, _) => Some(*id),
                        MemberListKey::Dm(_) => None,
                    },
                    channel_id: match &self.key {
                        MemberListKey::Room(_) => None,
                        MemberListKey::RoomChannel(_, _) => None,
                        MemberListKey::RoomThread(_, _, id) => Some(*id),
                        MemberListKey::Dm(id) => Some(*id),
                    },
                    ops,
                    groups: self.groups.clone(),
                });
            }
        }
    }

    pub async fn handle_sync(&mut self, event: MessageSync, snapshot: &RoomSnapshot) {
        match event {
            MessageSync::RoomMemberCreate { member, user }
            | MessageSync::RoomMemberUpdate { member, user } => {
                if self.key.room_id() == Some(member.room_id) {
                    let mut users_map = HashMap::new();
                    users_map.insert(user.id, user);
                    self.recalculate_member(member.user_id, &member, &users_map, snapshot)
                        .await;
                }
            }
            MessageSync::RoomMemberDelete { room_id, user_id } => {
                if self.key.room_id() == Some(room_id) {
                    self.remove_member(user_id);
                }
            }
            MessageSync::ThreadMemberUpsert {
                thread_id,
                added,
                removed,
                ..
            } => {
                if let MemberListKey::RoomThread(_, _, target_id) = &self.key {
                    if *target_id == thread_id {
                        for user_id in removed {
                            self.remove_member(user_id);
                        }
                        if !added.is_empty() {
                            for member in added {
                                if let Ok(user) =
                                    self.s.services().cache.user_get(member.user_id).await
                                {
                                    let mut users_map = HashMap::new();
                                    users_map.insert(user.id, user);
                                    if let Some(rm) = snapshot.members.get(&member.user_id) {
                                        self.recalculate_member(
                                            member.user_id,
                                            &rm.member,
                                            &users_map,
                                            snapshot,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                let old_key = self.user_to_key.get(&user_id).cloned();
                let is_online_old = old_key.as_ref().map_or(false, |key| match key.group {
                    MemberGroupInfo::Offline => false,
                    _ => true,
                });

                if is_online_old != presence.is_online() || old_key.is_none() {
                    if let Ok(mut user) = self.s.services().cache.user_get(user_id).await {
                        user.presence = presence.clone();
                        if let Some(_room_id) = self.key.room_id() {
                            if let Some(member) = snapshot.members.get(&user_id) {
                                let mut users_map = HashMap::new();
                                users_map.insert(user_id, user);
                                self.recalculate_member(
                                    user_id,
                                    &member.member,
                                    &users_map,
                                    snapshot,
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            MessageSync::UserUpdate { user } => {
                let user_id = user.id;
                if let Some(old_key) = self.user_to_key.get(&user_id) {
                    if old_key.name.as_ref() != user.name.as_str() {
                        if let Some(_room_id) = self.key.room_id() {
                            if let Some(member) = snapshot.members.get(&user_id) {
                                let mut users_map = HashMap::new();
                                users_map.insert(user_id, user.clone());
                                self.recalculate_member(
                                    user_id,
                                    &member.member,
                                    &users_map,
                                    snapshot,
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            MessageSync::RoleUpdate { role } => {
                if self.key.room_id() == Some(role.room_id) {
                    let _ = self.initialize(snapshot).await;
                }
            }
            MessageSync::RoleDelete { room_id, .. } => {
                if self.key.room_id() == Some(room_id) {
                    let _ = self.initialize(snapshot).await;
                }
            }
            MessageSync::RoleReorder { room_id, .. } => {
                if self.key.room_id() == Some(room_id) {
                    let _ = self.initialize(snapshot).await;
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                if self.key.room_id() == channel.room_id {
                    let _ = self.initialize(snapshot).await;
                }
            }
            _ => {}
        }
    }

    async fn recalculate_member(
        &mut self,
        user_id: UserId,
        member: &RoomMember,
        users_map: &HashMap<UserId, User>,
        snapshot: &RoomSnapshot,
    ) {
        if let Some(_room_id) = self.key.room_id() {
            let perms_calc = PermissionsCalculator {
                room_id: snapshot.room.id,
                owner_id: snapshot.room.owner_id,
                public: snapshot.room.public,
                room: Arc::new(snapshot.clone()),
            };
            let included = self.should_include(&user_id, member, &perms_calc, snapshot);

            let old_key = self.user_to_key.get(&user_id).cloned();

            if included {
                let new_key = self.calculate_key(&user_id, member, users_map, snapshot);
                let mut ops = Vec::new();

                if let Some(ok) = old_key {
                    let old_pos = self.members.range(..&ok).count() as u64;
                    self.members.remove(&ok);
                    ops.push(MemberListOp::Delete {
                        position: old_pos,
                        count: 1,
                    });
                }

                self.members.insert(new_key.clone(), user_id);
                self.user_to_key.insert(user_id, new_key.clone());
                self.recalculate_groups();

                let new_pos = self.members.range(..&new_key).count() as u64;
                let mut thread_member = None;
                if let MemberListKey::RoomThread(_, _, channel_id) = &self.key {
                    thread_member = self
                        .s
                        .data()
                        .thread_member_get(*channel_id, user_id)
                        .await
                        .ok();
                }

                ops.push(MemberListOp::Insert {
                    position: new_pos,
                    user_id,
                    room_member: Some(member.clone()),
                    thread_member,
                    user: Some(Box::new(users_map.get(&user_id).unwrap().clone())),
                });

                self.broadcast_ops(ops);
            } else if old_key.is_some() {
                self.remove_member(user_id);
            }
        }
    }

    pub fn remove_member(&mut self, user_id: UserId) {
        if let Some(key) = self.user_to_key.remove(&user_id) {
            let pos = self.members.range(..&key).count() as u64;
            self.members.remove(&key);
            self.recalculate_groups();
            self.broadcast_ops(vec![MemberListOp::Delete {
                position: pos,
                count: 1,
            }]);
        }
    }

    fn broadcast_ops(&self, ops: Vec<MemberListOp>) {
        let _ = self
            .events_tx
            .send(MemberListEvent::Broadcast(MessageSync::MemberListSync {
                user_id: UserId::new(), // dummy
                room_id: self.key.room_id(),
                channel_id: match &self.key {
                    MemberListKey::RoomThread(_, _, id) => Some(*id),
                    MemberListKey::Dm(id) => Some(*id),
                    _ => None,
                },
                ops,
                groups: self.groups.clone(),
            }));
    }

    async fn get_initial_ranges(
        &self,
        ranges: &[(u64, u64)],
        snapshot: &RoomSnapshot,
    ) -> Vec<MemberListOp> {
        let srv = self.s.services();

        let mut ops = Vec::new();
        let sorted_ids: Vec<_> = self.members.values().copied().collect();

        for (start, end) in ranges {
            let start = *start as usize;
            let end = (*end as usize).min(sorted_ids.len());
            if start >= end {
                continue;
            }

            let slice = &sorted_ids[start..end];
            let mut room_members = Vec::new();
            let mut thread_members = Vec::new();
            let mut users = Vec::new();

            for user_id in slice {
                if let Ok(mut u) = srv.cache.user_get(*user_id).await {
                    u.presence = srv.presence.get(*user_id);
                    users.push(u);
                }
                if let Some(_room_id) = self.key.room_id() {
                    if let Some(m) = snapshot.members.get(user_id) {
                        room_members.push(m.member.clone());
                    }
                }
                if let MemberListKey::RoomThread(_, _, channel_id) = &self.key {
                    if let Ok(m) = self.s.data().thread_member_get(*channel_id, *user_id).await {
                        thread_members.push(m);
                    }
                }
            }

            ops.push(MemberListOp::Sync {
                position: start as u64,
                items: slice.to_vec(),
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
                users: Some(users),
            });
        }
        ops
    }
}
