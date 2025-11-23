use common::v1::types::{
    MemberListGroup, MemberListGroupId, MemberListOp, MessageSync, Role, RoleId, RoomMembership,
    ThreadMembership, UserId,
};
use tracing::warn;

use crate::services::members::util::MemberListKey;

pub struct MemberList2 {
    pub key: MemberListKey,
    pub roles: Vec<Role>,
    pub groups: Vec<MemberList2Group>,
}

pub struct MemberList2Group {
    pub id: MemberListGroupId,
    pub items: Vec<MemberList2Item>,
}

/// a single member in a member list
#[derive(Clone, PartialEq, Eq)]
pub struct MemberList2Item {
    pub user_id: UserId,
    pub name: String,
    pub nick: Option<String>,
    pub roles: Vec<RoleId>,
}

impl MemberList2 {
    /// handle a sync event and calculate what operations need to be applied
    pub fn process(&mut self, event: &MessageSync) -> Vec<MemberListOp> {
        match event {
            MessageSync::ChannelUpdate { channel } => {
                // handle view overwrite update
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
                    self.upsert_user(MemberList2Item {
                        user_id: member.user_id,
                        // FIXME: get actual user name (maybe return User object in RoomMemberUpsert?)
                        name: "user name".to_string(),
                        nick: member.override_name.clone(),
                        roles: member.roles.clone(),
                    })
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
                    self.upsert_user(MemberList2Item {
                        user_id: member.user_id,
                        // FIXME: get actual user name (maybe return User object in RoomMemberUpsert?)
                        name: "user name".to_string(),
                        // FIXME: store room member nickname/roles
                        nick: None,
                        roles: vec![],
                    })
                }
            }
            MessageSync::RoleUpdate { role } => {
                if self.key.room_id != Some(role.room_id) {
                    return vec![];
                }

                let Some(existing) = self.roles.iter().find(|r| r.id == role.id) else {
                    warn!("got RoleUpdate for role we dont have");
                    return vec![];
                };

                if existing.hoist == role.hoist {
                    // no change
                    return vec![];
                }

                if role.hoist {
                    // role is hoisted
                    // 1. get members with this role as their top hoisted role
                    // 2. issue deletes for all of those members
                    // 3. create new group for members with those roles
                    todo!()
                } else {
                    // role is now not hoisted
                    self.remove_group(MemberListGroupId::Role(role.id))
                }
            }
            MessageSync::RoleDelete { room_id, role_id } => {
                if self.key.room_id != Some(*room_id) {
                    return vec![];
                }

                // role has been deleted
                // TODO: remove role from self.roles
                self.remove_group(MemberListGroupId::Role(*role_id))
            }
            MessageSync::RoleReorder { room_id, roles } => {
                // handle role reorder
                todo!()
            }
            MessageSync::UserUpdate { user } => {
                // handle name change
                todo!()
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                // handle offline -> online
                // handle online -> offline
                todo!()
            }
            _ => vec![],
        }
    }

    /// get a list of Sync ops for these ranges. used when initially syncing a member list
    pub fn get_initial_ranges(&self, ranges: &[(u64, u64)]) -> Vec<MemberListOp> {
        todo!()
    }

    /// get a list of groups for this member list
    pub fn groups(&self) -> Vec<MemberListGroup> {
        self.groups
            .iter()
            .map(|g| MemberListGroup {
                id: g.id,
                count: g.items.len() as u64,
            })
            .collect()
    }

    /// remove a user from this list
    // NOTE: will generally only emit one member list op (Delete)
    fn remove_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
        todo!()
    }

    /// insert a user into this list. if this user already exists, this may return nothing or a delete/insert op pair to move this user
    fn upsert_user(&mut self, item: MemberList2Item) -> Vec<MemberListOp> {
        todo!()
    }

    /// get a user
    fn get_user(&self, user_id: UserId) -> Option<MemberList2Item> {
        todo!()
    }

    /// remove a group
    fn remove_group(&mut self, group: MemberListGroupId) -> Vec<MemberListOp> {
        // 0. check if group exists
        // 1. issue delete for group
        // 2. remove group from self.groups
        // 3. issue inserts for every member in that group
        todo!()
    }
}
