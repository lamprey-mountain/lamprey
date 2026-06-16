use std::collections::{BTreeMap, HashMap, hash_map::Entry};

use common::{
    v1::types::{MemberListGroup, MemberListOp, MessageSync, RoomMember, User},
    v2::types::{ChannelId, RoomId, UserId},
};
use tokio::sync::broadcast;

use crate::{
    prelude::*,
    services::member_lists::{MemberGroupInfo, MemberKey, MemberListSync},
};

// TODO: copy member list logic to common or core?
/// a room's member list
#[derive(Debug)]
pub struct List {
    /// ordered map of members for range queries and position tracking
    members: BTreeMap<MemberKey, UserId>,

    /// reverse lookup: UserId -> MemberKey
    // PERF: share between member lists (store in room/roomdata?)
    user_to_key: HashMap<UserId, MemberKey>,

    /// group summaries (id and count)
    groups: BTreeMap<MemberGroupInfo, MemberListGroup>,
}

/// a handle for interacting with a room's member list
pub struct ListHandle {
    //
}

#[derive(Debug, Clone)]
pub enum ListTarget {
    Room,
    Channel(ChannelId),
}

impl List {
    // let (send, recv) = tokio::sync::broadcast::channel(capacity);
    // list is idle if send.receiver_count() == 0;
    // if idle for too long, clean up the list

    /// handle a `MessageSync` event
    ///
    /// assumes the event is for this list
    fn handle_sync(&mut self, events: &[MessageSync]) {
        let mut ops = vec![];
        for event in events {
            self.handle_sync_inner(event, &mut ops);
        }
        // TODO: broadcast sync
        // MemberListSync::Sync {
        //     room_id: (),
        //     channel_id: (),
        //     ops,
        //     groups: (),
        // };
    }

    fn handle_sync_inner(&mut self, event: &MessageSync, ops: &mut Vec<MemberListOp>) {
        match event {
            MessageSync::RoomMemberCreate { member, user }
            | MessageSync::RoomMemberUpdate { member, user } => {
                let can_view = self.can_view(user, member);

                let old_key = self.user_to_key.get(&user.id);
                match (old_key, can_view) {
                    (None, true) => {
                        // add member
                        let key = self.calculate_key(&user, &member);
                        self.members.insert(key.clone(), user.id);
                        self.groups
                            .entry(key.group.clone())
                            .and_modify(|g| g.count += 1)
                            .or_insert_with(|| MemberListGroup {
                                id: key.group.into(),
                                count: 1,
                            });
                        let op = MemberListOp::Insert {
                            position: todo!(),
                            user_id: user.id,
                            room_member: todo!(),
                            thread_member: todo!(),
                            user: todo!(),
                        };
                        ops.push(op);
                    }
                    (Some(_), false) => {
                        // remove member
                        if let Some(key) = self.user_to_key.remove(&user.id) {
                            let pos = self.members.range(..&key).count() as u64;
                            self.members.remove(&key);
                            self.groups.get_mut(&key.group).map(|k| k.count -= 1);
                            let op = MemberListOp::Delete {
                                position: pos,
                                count: 1,
                            };
                            ops.push(op);
                        }
                    }
                    (Some(_), true) => {
                        // reorder member
                        let key = self.calculate_key(&user, &member);

                        // PERF: don't call entry again, merge this call with first `let old_key =`
                        let has_cached = match self.user_to_key.entry(user.id) {
                            Entry::Occupied(mut e) => {
                                let old_key = e.get();
                                if old_key == &key {
                                    // skip updating
                                    return;
                                } else {
                                    // member already exists in the list, update their position
                                    let old_pos = self.members.range(..old_key).count() as u64;
                                    ops.push(MemberListOp::Delete {
                                        position: old_pos,
                                        count: 1,
                                    });

                                    self.groups.get_mut(&old_key.group).map(|k| k.count -= 1);
                                    self.members.remove(old_key);
                                    self.members.insert(key.clone(), user.id);
                                    e.insert(key.clone());
                                    true
                                }
                            }
                            Entry::Vacant(e) => {
                                // member doesn't exist in the list, insert them
                                e.insert(key.clone());
                                self.members.insert(key.clone(), user.id);
                                self.user_to_key.insert(user.id, key.clone());
                                false
                            }
                        };

                        self.groups
                            .entry(key.group.clone())
                            .and_modify(|g| g.count += 1)
                            .or_insert_with(|| MemberListGroup {
                                id: key.group.into(),
                                count: 1,
                            });

                        let pos = self.members.range(..key).count() as u64;
                        ops.push(MemberListOp::Insert {
                            position: pos,
                            user_id: user.id,
                            room_member: if has_cached {
                                None
                            } else {
                                Some(member.clone())
                            },
                            thread_member: if has_cached {
                                None
                            } else {
                                todo!("handle thread member list + room member update")
                            },
                            user: if has_cached {
                                None
                            } else {
                                Some(Box::new(user.clone()))
                            },
                        })
                    }
                    (None, false) => {}
                }
            }
            MessageSync::RoomMemberDelete { user_id, .. } => {
                if let Some(key) = self.user_to_key.remove(&user_id) {
                    let pos = self.members.range(..&key).count() as u64;
                    self.members.remove(&key);
                    self.groups.get_mut(&key.group).map(|k| k.count -= 1);
                    let op = MemberListOp::Delete {
                        position: pos,
                        count: 1,
                    };
                    ops.push(op);
                }
            }
            MessageSync::ThreadMemberUpsert { .. } => {
                todo!()
            }
            MessageSync::PresenceUpdate { .. } => {
                todo!()
            }
            MessageSync::UserUpdate { .. } => {
                todo!()
            }
            // RoleCreate isn't handled since the member list wouldn't update until a member was assigned that role anyways
            MessageSync::RoleUpdate { .. } => {
                todo!()
            }
            MessageSync::RoleDelete { .. } => {
                todo!()
            }
            MessageSync::RoleReorder { .. } => {
                todo!()
            }
            MessageSync::ChannelUpdate { .. } => {
                todo!("handle permission overwrite updates")
            }
            _ => {}
        }
    }

    /// calculate the member key (sorting key) for a room member
    fn calculate_key(&self, _user: &User, _member: &RoomMember) -> MemberKey {
        todo!()
    }

    /// calculate whether this room member can view this member list
    fn can_view(&self, _user: &User, _member: &RoomMember) -> bool {
        todo!()
    }
}

impl ListHandle {
    pub fn room_id(&self) -> RoomId {
        todo!()
    }

    pub fn target(&self) -> ListTarget {
        todo!()
    }

    // /// get initial ranges
    // fn initial_ranges(&mut self, ranges: &[(u64, u64)]) -> MemberListSync {
    //     todo!()
    // }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<MemberListSync>> {
        todo!()
    }
}
