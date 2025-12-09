use common::v1::types::{
    ChannelId, MemberListGroup, MemberListGroupId, MemberListOp, MessageSync, Permission,
    PermissionOverwrite, PermissionOverwriteType, RoleId, RoomId, RoomMember, UserId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberListTarget {
    Room(RoomId),
    Channel(ChannelId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberGroupInfo {
    Hoisted { role_id: RoleId, role_position: u64 },
    Online,
    Offline,
}

/// for deduplicating member lists
// TODO: use permission overwrites (for view permission) instead of creating a list per channel
// unlike discord, this needs to handle permission overwrite inheritance
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MemberListKey {
    pub room_id: Option<RoomId>,
    pub channel_id: Option<ChannelId>,
}

// maybe redo member list key into an enum
// enum MemberListKey {
//     Room(RoomId),
//     RoomChannel(RoomId, ChannelId),
//     RoomThread(RoomId, ChannelId),
//     Dm(ChannelId),
// }
//
// i also still want to dedup lists by permission overwrites, so two channels with the same set of permissions get deduped
// pub enum MemberListKey {
//     /// a room member list
//     Room {
//         room_id: RoomId,

//         // empty for the main list
//         overwrites: Vec<Vec<PermissionOverwrite>>,
//     },

//     /// (group) direct messages
//     // maybe since recipients exists i dont need to have this at all?
//     Dm { channel_id: ChannelId },
// }

impl PartialOrd for MemberGroupInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberGroupInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            // hoisted roles are ordered by position (lower position = higher precedence = Less)
            (
                MemberGroupInfo::Hoisted {
                    role_position: a, ..
                },
                MemberGroupInfo::Hoisted {
                    role_position: b, ..
                },
            ) => a.cmp(b),

            // hoisted roles come before online and offline
            (MemberGroupInfo::Hoisted { .. }, MemberGroupInfo::Online) => Ordering::Less,
            (MemberGroupInfo::Hoisted { .. }, MemberGroupInfo::Offline) => Ordering::Less,

            // Online comes before offline
            (MemberGroupInfo::Online, MemberGroupInfo::Hoisted { .. }) => Ordering::Greater,
            (MemberGroupInfo::Online, MemberGroupInfo::Online) => Ordering::Equal,
            (MemberGroupInfo::Online, MemberGroupInfo::Offline) => Ordering::Less,

            // Offline comes after everything else
            (MemberGroupInfo::Offline, MemberGroupInfo::Hoisted { .. }) => Ordering::Greater,
            (MemberGroupInfo::Offline, MemberGroupInfo::Online) => Ordering::Greater,
            (MemberGroupInfo::Offline, MemberGroupInfo::Offline) => Ordering::Equal,
        }
    }
}

impl MemberListKey {
    pub fn room(room_id: RoomId) -> Self {
        Self {
            room_id: Some(room_id),
            channel_id: None,
        }
    }

    pub fn channel(channel_id: ChannelId) -> Self {
        Self {
            room_id: None,
            channel_id: Some(channel_id),
        }
    }
}

impl From<MemberGroupInfo> for MemberListGroupId {
    fn from(value: MemberGroupInfo) -> Self {
        match value {
            MemberGroupInfo::Hoisted {
                role_id,
                role_position: _,
            } => MemberListGroupId::Role(role_id),
            MemberGroupInfo::Online => MemberListGroupId::Online,
            MemberGroupInfo::Offline => MemberListGroupId::Offline,
        }
    }
}

#[derive(Debug, Default)]
pub struct MemberListVisibility {
    /// list of permission overwrites in order from topmost parent to the channel itself
    pub overwrites: Vec<Vec<PermissionOverwrite>>,
}

impl MemberListVisibility {
    /// check if this member can view a channel with this set of overwrites. has_base is if the member can view all channels by default.
    // TODO: dedup this code with canonical permission logic in services/permission.rs
    pub fn visible_to(&self, member: &RoomMember, has_base: bool) -> bool {
        let mut has_view = has_base;

        // apply each overwrite in order
        for ow_set in &self.overwrites {
            // apply role allow overwrites
            for ow in ow_set {
                if ow.ty != PermissionOverwriteType::Role {
                    continue;
                }

                if !member.roles.contains(&ow.id.into()) {
                    continue;
                }

                if ow.allow.contains(&Permission::ViewChannel) {
                    has_view = true;
                }
            }

            // apply role deny overwrites
            for ow in ow_set {
                if ow.ty != PermissionOverwriteType::Role {
                    continue;
                }

                if !member.roles.contains(&ow.id.into()) {
                    continue;
                }

                if ow.deny.contains(&Permission::ViewChannel) {
                    has_view = false;
                }
            }

            // apply user allow overwrites
            for ow in ow_set {
                if ow.ty != PermissionOverwriteType::User {
                    continue;
                }

                if ow.id != *member.user_id {
                    continue;
                }

                if ow.allow.contains(&Permission::ViewChannel) {
                    has_view = true;
                }
            }

            // apply user deny overwrites
            for ow in ow_set {
                if ow.ty != PermissionOverwriteType::User {
                    continue;
                }

                if ow.id != *member.user_id {
                    continue;
                }

                if ow.deny.contains(&Permission::ViewChannel) {
                    has_view = false;
                }
            }
        }

        has_view
    }
}

/// minimal member list sync payload for broadcasting
#[derive(Debug, Clone)]
pub struct MemberListSync {
    pub key: MemberListKey,
    pub ops: Vec<MemberListOp>,
    pub groups: Vec<MemberListGroup>,
}

pub struct Ranges {
    pub inner: Vec<(u64, u64)>,
}

impl Ranges {
    pub fn new(inner: Vec<(u64, u64)>) -> Self {
        Self { inner }
    }

    /// return if a given index is inside the ranges
    pub fn contains(&self, idx: u64) -> bool {
        for &(start, end) in &self.inner {
            if idx >= start && idx < end {
                return true;
            }
        }

        false
    }

    /// return if a given range intersects with any of the ranges
    pub fn intersects(&self, other_start: u64, other_end: u64) -> bool {
        for &(start, end) in &self.inner {
            if start.max(other_start) < end.min(other_end) {
                return true;
            }
        }
        false
    }
}

impl MemberListSync {
    pub fn into_sync_message(self, user_id: UserId) -> MessageSync {
        MessageSync::MemberListSync {
            user_id,
            room_id: self.key.room_id,
            channel_id: self.key.channel_id,
            ops: self.ops,
            groups: self.groups,
        }
    }

    /// filter/split ops to only include those in these ranges
    pub fn mask(self, ranges: &[(u64, u64)]) -> MemberListSync {
        let mut ops = vec![];
        let ranges = Ranges::new(ranges.to_vec());

        for op in self.ops {
            match op {
                MemberListOp::Sync {
                    position,
                    room_members,
                    thread_members,
                    users,
                } => {
                    let op_end = position + users.len() as u64;

                    for &(start, end) in &ranges.inner {
                        let intersect_start = position.max(start);
                        let intersect_end = op_end.min(end);

                        if intersect_start < intersect_end {
                            let slice_start = (intersect_start - position) as usize;
                            let slice_end = (intersect_end - position) as usize;

                            let new_users = users[slice_start..slice_end].to_vec();

                            let new_room_members = room_members.as_ref().map(|v| {
                                v.iter()
                                    .filter(|m| new_users.iter().any(|u| u.id == m.user_id))
                                    .cloned()
                                    .collect()
                            });

                            let new_thread_members = thread_members.as_ref().map(|v| {
                                v.iter()
                                    .filter(|m| new_users.iter().any(|u| u.id == m.user_id))
                                    .cloned()
                                    .collect()
                            });

                            ops.push(MemberListOp::Sync {
                                position: intersect_start,
                                room_members: new_room_members,
                                thread_members: new_thread_members,
                                users: new_users,
                            });
                        }
                    }
                }
                MemberListOp::Delete { position, count } => {
                    if ranges.intersects(position, position + count) {
                        ops.push(MemberListOp::Delete { position, count });
                    }
                }
                MemberListOp::Insert {
                    position,
                    room_member,
                    thread_member,
                    user,
                } => {
                    if ranges.contains(position) {
                        ops.push(MemberListOp::Insert {
                            position,
                            room_member,
                            thread_member,
                            user,
                        });
                    }
                }
            }
        }

        MemberListSync {
            key: self.key,
            ops,
            groups: self.groups,
        }
    }
}