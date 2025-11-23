use std::collections::HashMap;

use common::v1::types::{
    presence::Presence, MemberListGroup, MemberListGroupId, MemberListOp, MessageSync,
    PaginationQuery, Role, RoleId, RoomMember, RoomMembership, ThreadMember, ThreadMembership,
    User, UserId,
};
use tracing::warn;

use crate::{
    services::members::util::{MemberGroupInfo, MemberListKey},
    Result, ServerState,
};

pub struct MemberList2 {
    pub key: MemberListKey,
    pub roles: Vec<Role>,
    pub groups: Vec<MemberList2Group>,

    // TODO: use Arc for everything?
    pub presences: HashMap<UserId, Presence>,
    pub room_members: HashMap<UserId, RoomMember>,
    pub thread_members: HashMap<UserId, ThreadMember>,
    pub users: HashMap<UserId, User>,
}

pub struct MemberList2Group {
    pub info: MemberGroupInfo,
    pub users: Vec<UserId>,
}

impl MemberList2 {
    /// create a new member list. fetches data from server state.
    pub async fn new_from_server(key: MemberListKey, s: &ServerState) -> Result<Self> {
        let mut me = Self {
            key: key.clone(),
            roles: vec![],
            groups: vec![],
            presences: HashMap::new(),
            room_members: HashMap::new(),
            thread_members: HashMap::new(),
            users: HashMap::new(),
        };
        let data = s.data();
        let srv = s.services();

        if let Some(room_id) = key.room_id {
            let roles = data
                .role_list(
                    room_id,
                    PaginationQuery {
                        limit: Some(1024),
                        ..Default::default()
                    },
                )
                .await?;
            me.roles = roles.items;
            for m in data.room_member_list_all(room_id).await? {
                me.room_members.insert(m.user_id, m);
            }
        }

        if let Some(channel_id) = key.channel_id {
            let channel = srv.channels.get(channel_id, None).await?;
            if channel.ty.is_thread() {
                for m in data.thread_member_list_all(channel_id).await? {
                    me.thread_members.insert(m.user_id, m);
                }
            }
        }

        let user_ids: Vec<_> = me
            .room_members
            .keys()
            .chain(me.thread_members.keys())
            .copied()
            .collect();
        for u in srv.users.get_many(&user_ids).await? {
            me.presences.insert(u.id, u.presence.clone());
            me.users.insert(u.id, u);
        }

        // TODO: build initial groups

        Ok(me)
    }

    /// handle a sync event and calculate what operations need to be applied
    pub fn process(&mut self, event: &MessageSync) -> Vec<MemberListOp> {
        match event {
            MessageSync::ChannelUpdate { channel: _ } => {
                // handle view overwrite update
                // handle inheritance (eg. Category -> Text -> ThreadPublic)
                todo!()
            }
            MessageSync::RoomMemberUpsert { member } => {
                if self.key.room_id != Some(member.room_id) {
                    return vec![];
                }

                if member.membership == RoomMembership::Leave {
                    // member left the room
                    self.remove_user(member.user_id)
                } else {
                    // member joined, changed roles, or changed override_name
                    self.room_members.insert(member.user_id, member.clone());
                    if self.users.contains_key(&member.user_id) {
                        self.recalculate_user(member.user_id)
                    } else {
                        warn!(
                            "RoomMemberUpsert for user {} without User object, can't update list",
                            member.user_id
                        );
                        vec![]
                    }
                }
            }
            MessageSync::ThreadMemberUpsert { member } => {
                if self.key.channel_id != Some(member.thread_id) {
                    return vec![];
                }

                if member.membership == ThreadMembership::Leave {
                    // member left thread
                    self.remove_user(member.user_id)
                } else {
                    // member joined thread
                    self.thread_members.insert(member.user_id, member.clone());
                    if self.users.contains_key(&member.user_id) {
                        self.recalculate_user(member.user_id)
                    } else {
                        warn!(
                            "ThreadMemberUpsert for user {} without User object, can't update list",
                            member.user_id
                        );
                        vec![]
                    }
                }
            }
            MessageSync::RoleUpdate { role } => {
                if self.key.room_id != Some(role.room_id) {
                    return vec![];
                }

                let Some(existing) = self.roles.iter_mut().find(|r| r.id == role.id) else {
                    warn!("got RoleUpdate for role we dont have");
                    return vec![];
                };

                let old_hoist = existing.hoist;
                *existing = role.clone();

                if old_hoist == role.hoist {
                    // no change that affects member list groups
                    return vec![];
                }

                if role.hoist {
                    // role is now hoisted, some members might move to this new hoisted group
                    // we need to find all members that have this role and recalculate their group
                    let members_with_role: Vec<_> = self
                        .room_members
                        .values()
                        .filter(|rm| rm.roles.contains(&role.id))
                        .map(|rm| rm.user_id)
                        .collect();

                    let mut ops = vec![];
                    for user_id in members_with_role {
                        ops.extend(self.recalculate_user(user_id));
                    }
                    ops
                } else {
                    // role is no longer hoisted
                    self.remove_group(MemberListGroupId::Role(role.id))
                }
            }
            MessageSync::RoleDelete { room_id, role_id } => {
                if self.key.room_id != Some(*room_id) {
                    return vec![];
                }

                self.roles.retain(|r| r.id != *role_id);
                self.remove_group(MemberListGroupId::Role(*role_id))
            }
            MessageSync::RoleReorder {
                room_id: _,
                roles: _,
            } => {
                // handle role reorder
                todo!()
            }
            MessageSync::UserUpdate { user } => {
                let old_user_name = self.users.get(&user.id).map(|u| u.name.clone());
                self.users.insert(user.id, user.clone());

                // user's name changed
                if old_user_name.as_deref() != Some(user.name.as_str()) {
                    let is_in_list = if self.key.room_id.is_some() {
                        self.room_members.contains_key(&user.id)
                    } else if self.key.channel_id.is_some() {
                        self.thread_members.contains_key(&user.id)
                    } else {
                        false
                    };

                    if is_in_list {
                        return self.recalculate_user(user.id);
                    }
                }

                vec![]
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                self.presences.insert(*user_id, presence.clone());

                let is_in_list = if self.key.room_id.is_some() {
                    self.room_members.contains_key(user_id)
                } else if self.key.channel_id.is_some() {
                    self.thread_members.contains_key(user_id)
                } else {
                    false
                };

                // user may have gone online or offline
                if is_in_list {
                    self.recalculate_user(*user_id)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    /// get a list of Sync ops for these ranges. used when initially syncing a member list
    pub fn get_initial_ranges(&self, ranges: &[(u64, u64)]) -> Vec<MemberListOp> {
        let sorted_members: Vec<UserId> = self
            .groups
            .iter()
            .flat_map(|g| g.users.iter().copied())
            .collect();

        let mut ops = vec![];

        for (start, end) in ranges {
            let start = *start as usize;
            let end = (*end as usize).min(sorted_members.len());

            if start >= end {
                continue;
            }

            let slice = &sorted_members[start..end];

            let mut room_members = vec![];
            let mut thread_members = vec![];
            let mut users = vec![];

            // TODO: verify that all three vecs are the same length
            for user_id in slice {
                if let Some(rm) = self.room_members.get(user_id) {
                    room_members.push(rm.clone());
                }
                if let Some(tm) = self.thread_members.get(user_id) {
                    thread_members.push(tm.clone());
                }
                if let Some(u) = self.users.get(user_id) {
                    users.push(u.clone());
                } else {
                    warn!("user {} not found in users map", user_id);
                }
            }

            ops.push(MemberListOp::Sync {
                position: start as u64,
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

        ops
    }

    /// get a list of groups for this member list
    pub fn groups(&self) -> Vec<MemberListGroup> {
        self.groups
            .iter()
            .map(|g| MemberListGroup {
                id: g.info.into(),
                count: g.users.len() as u64,
            })
            .filter(|g| g.count != 0)
            .collect()
    }

    /// remove a user from this list
    // NOTE: will generally only emit one member list op (Delete)
    fn remove_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
        self.presences.remove(&user_id);
        self.room_members.remove(&user_id);
        self.thread_members.remove(&user_id);
        self.users.remove(&user_id);

        if let Some((group_idx, item_idx)) = self.find_user(user_id) {
            let position: usize = self.groups[..group_idx]
                .iter()
                .map(|g| g.users.len())
                .sum::<usize>()
                + item_idx;

            self.groups[group_idx].users.remove(item_idx);

            if self.groups[group_idx].users.is_empty() {
                self.groups.remove(group_idx);
            }

            vec![MemberListOp::Delete {
                position: position as u64,
                count: 1,
            }]
        } else {
            vec![]
        }
    }

    /// get the group index and index of the user inside that group
    fn find_user(&self, user_id: UserId) -> Option<(usize, usize)> {
        for (group_idx, group) in self.groups.iter().enumerate() {
            if let Some(user_idx) = group.users.iter().position(|&id| id == user_id) {
                return Some((group_idx, user_idx));
            }
        }
        None
    }

    /// get the index of a group
    fn find_group(&self, group_id: MemberListGroupId) -> Option<usize> {
        self.groups
            .iter()
            .position(|g| MemberListGroupId::from(g.info) == group_id)
    }

    /// get the group id for a given member
    // TODO: check self.presences, remove is_online
    fn get_member_group_id(&self, user_id: UserId, is_online: bool) -> MemberListGroupId {
        if is_online {
            let roles_map: std::collections::HashMap<_, _> =
                self.roles.iter().map(|r| (r.id, r)).collect();

            let member_roles = self
                .room_members
                .get(&user_id)
                .map(|rm| rm.roles.as_slice())
                .unwrap_or(&[]);

            if let Some(role) = member_roles
                .iter()
                .filter_map(|role_id| roles_map.get(role_id))
                .filter(|r| r.hoist)
                .min_by_key(|r| r.position)
            {
                MemberListGroupId::Role(role.id)
            } else {
                MemberListGroupId::Online
            }
        } else {
            MemberListGroupId::Offline
        }
    }

    /// recalculate a user's position in list
    ///
    /// - if this user already exists, this may return nothing or a delete/insert op pair to move this user
    /// - if the user doesnt already exist, this will return a single op
    fn recalculate_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
        let mut ops = vec![];

        let user = if let Some(user) = self.users.get(&user_id) {
            user.to_owned()
        } else {
            warn!(
                "upsert_user called for user {} but user object not found",
                user_id
            );
            return vec![];
        };

        // remove existing item, if it exists
        if let Some((group_idx, item_idx)) = self.find_user(user_id) {
            let old_pos: usize = self.groups[..group_idx]
                .iter()
                .map(|g| g.users.len())
                .sum::<usize>()
                + item_idx;
            self.groups[group_idx].users.remove(item_idx);

            ops.push(MemberListOp::Delete {
                position: old_pos as u64,
                count: 1,
            });
        }

        let is_online = self
            .presences
            .get(&user_id)
            .map(|p| p.is_online())
            .unwrap_or(false);
        let group_id = self.get_member_group_id(user_id, is_online);

        let group_idx = self.insert_group(group_id);

        // find position to insert within group, maintaining sort order
        let group = &mut self.groups[group_idx];

        let get_display_name =
            |uid: &UserId,
             users: &HashMap<UserId, User>,
             room_members: &HashMap<UserId, RoomMember>| {
                let nick = room_members
                    .get(uid)
                    .and_then(|rm| rm.override_name.as_deref());
                let name = users.get(uid).map(|u| u.name.as_str());
                nick.or(name).unwrap_or_default().to_owned()
            };

        let display_name = get_display_name(&user_id, &self.users, &self.room_members);

        let item_idx = group
            .users
            .binary_search_by(|uid| {
                get_display_name(uid, &self.users, &self.room_members).cmp(&display_name)
            })
            .unwrap_or_else(|e| e);

        group.users.insert(item_idx, user_id);

        // calculate absolute position of new item
        let new_pos = self.groups[..group_idx]
            .iter()
            .map(|g| g.users.len())
            .sum::<usize>()
            + item_idx;

        ops.push(MemberListOp::Insert {
            position: new_pos as u64,
            room_member: self.room_members.get(&user_id).cloned(),
            thread_member: self.thread_members.get(&user_id).cloned(),
            user: Box::new(user.clone()),
        });

        ops
    }

    /// create a new group if it doesnt exist. returns the group index.
    fn insert_group(&mut self, group_id: MemberListGroupId) -> usize {
        let new_group_info = match group_id {
            MemberListGroupId::Online => MemberGroupInfo::Online,
            MemberListGroupId::Offline => MemberGroupInfo::Offline,
            MemberListGroupId::Role(role_id) => {
                let role = self
                    .roles
                    .iter()
                    .find(|r| r.id == role_id)
                    .expect("role doesnt exist");
                MemberGroupInfo::Hoisted {
                    role_id,
                    role_position: role.position,
                }
            }
        };

        if let Some(pos) = self.groups.iter().position(|g| g.info == new_group_info) {
            return pos;
        }

        let insert_idx = self
            .groups
            .binary_search_by(|group| group.info.cmp(&new_group_info))
            .unwrap_or_else(|e| e);

        self.groups.insert(
            insert_idx,
            MemberList2Group {
                info: new_group_info,
                users: vec![],
            },
        );

        insert_idx
    }

    /// remove a group and re-insert its members
    fn remove_group(&mut self, group_id: MemberListGroupId) -> Vec<MemberListOp> {
        let mut ops = vec![];
        if let Some(group_idx) = self.find_group(group_id) {
            let group = self.groups.remove(group_idx);
            let position = self.groups[..group_idx]
                .iter()
                .map(|g| g.users.len())
                .sum::<usize>() as u64;

            // delete all members in the group
            if !group.users.is_empty() {
                ops.push(MemberListOp::Delete {
                    position,
                    count: group.users.len() as u64,
                });
            }

            // reinsert the users in the correct group
            for user_id in group.users {
                ops.extend(self.recalculate_user(user_id));
            }
        }
        ops
    }
}
