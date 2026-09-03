//! Service for managing member lists

use common::v2::types::{ConnectionId, RoomId};
use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::prelude::*;
use crate::services::member_lists::actor::MemberListHandle;
use crate::services::member_lists::{
    actor::{MemberListCommand, MemberListEvent},
    util::{MemberListKey, MemberListKey1},
    visibility::ListVisibility,
};
use crate::services::rooms::actor::{MemberListCommandMsg, MemberListSubscribeMsg};
use crate::services::rooms::{RoomActor, RoomHandle};

pub mod actor;
pub mod syncer;
pub mod util; // TODO: deprecate and remove; move any useful types to common/core
pub mod visibility;

/// Service for managing member lists
pub struct ServiceMemberLists {
    globals: Globals,

    // TODO: use this
    lists: DashMap<ListKey, MemberListHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListKey {
    room_id: RoomId,
    visibility: visibility::ListVisibility,
}

impl ServiceMemberLists {
    /// Create a new member lists service
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            lists: DashMap::new(),
        }
    }

    /// Lookup a member list key from an API key
    async fn lookup_member_key(&self, key1: MemberListKey1) -> Result<MemberListKey> {
        let srv = self.globals.services();
        match key1 {
            MemberListKey1::Room(room_id) => Ok(MemberListKey::Room(room_id)),
            MemberListKey1::RoomChannel(room_id, channel_id) => {
                let chan = srv.channels.get(channel_id, None).await?;
                if chan.is_thread() && chan.ty.member_list_uses_thread_members() {
                    return Ok(MemberListKey::RoomThread(
                        room_id,
                        ListVisibility::default(),
                        channel_id,
                    ));
                }
                let overwrites = srv.channels.fetch_overwrite_ancestors(channel_id).await?;
                let visibility = ListVisibility::from_overwrites(room_id, overwrites);
                Ok(MemberListKey::RoomChannel(room_id, visibility))
            }
            MemberListKey1::DmChannel(channel_id) => Ok(MemberListKey::Dm(channel_id)),
        }
    }

    /// Ensure a member list exists and return its handle
    async fn ensure(&self, key: MemberListKey) -> Result<Arc<MemberListHandle>> {
        let room_id = key
            .room_id()
            .ok_or(crate::Error::BadStatic("DM member lists not yet sharded"))?;

        let get_handle = || {
            self.globals
                .services()
                .rooms
                .actors
                .try_get_with(room_id, || {
                    Ok::<RoomHandle, crate::Error>(RoomActor::spawn_room(
                        room_id,
                        self.globals.clone(),
                    ))
                })
                .map_err(|e| e.fake_clone())
        };

        let mut room_handle = get_handle()?;

        // Try to send the subscribe command; if it fails, the actor is dead
        // Evict the dead actor and retry once
        let result = room_handle
            .actor_ref
            .ask(MemberListSubscribeMsg { key: key.clone() })
            .send()
            .await;

        let actual_tx = match result {
            Ok(tx) => tx,
            _ => {
                // Actor is dead or failed, evict it
                self.globals.services().rooms.unload_cache(room_id).await;

                // Get a fresh actor
                room_handle = get_handle()?;

                room_handle
                    .actor_ref
                    .ask(MemberListSubscribeMsg { key: key.clone() })
                    .send()
                    .await
                    .map_err(|_| {
                        crate::Error::Internal("failed to subscribe to member list".to_string())
                    })?
            }
        };

        Ok(Arc::new(MemberListHandle {
            actor_ref: room_handle.actor_ref.clone(),
            key,
            events_tx: actual_tx,
        }))
    }

    // TODO(?): replace fn ensure() with this?
    // pub fn ensure_handle(
    //     &self,
    //     room_id: RoomId,
    //     channel_id: Option<ChannelId>,
    // ) -> actor::ListHandle {
    //     todo!()
    // }

    /// Create a new syncer for a connection
    pub fn create_syncer(&self, connection_id: ConnectionId) -> syncer::MemberListSyncer {
        syncer::MemberListSyncer::new(self.globals.clone(), connection_id)
    }

    // TODO: remove this
    /// Start background tasks for the service
    pub fn start_background_tasks(&self) {
        // No longer needed as RoomActor handles its own events
    }
}

#[cfg(any())]
mod next {
    // TODO: use the types here for the member list service

    pub mod actor {
        use std::collections::btree_map::Entry;
        use std::collections::{BTreeMap, HashMap};

        use common::v1::types::{MemberListOp, RoomMember, User};
        use common::v2::types::ChannelId;
        use common::{util::member_list::MemberKey, v1::types::MessageSync, v2::types::UserId};

        use crate::prelude::*;
        use crate::services::cache::RoomHandle;
        use crate::services::member_lists::util::MemberGroupInfo;

        // TODO: copy member list logic to common or core?
        // TODO(future): dm/gdm member list
        /// a member list.
        ///
        /// each list is currently tied to a room. in the future, lists *may* exist for dm/gdm channels too.
        #[derive(Debug)]
        pub struct List {
            room: ListRoom,

            /// group summaries (id and count)
            groups: BTreeMap<MemberGroupInfo, MemberListGroup>,
        }

        /// list data for a room
        pub struct ListRoom {
            room: RoomHandle,

            /// ordered map of members for range queries and position tracking
            members: BTreeMap<MemberKey, UserId>,

            /// reverse lookup: UserId -> MemberKey
            user_to_key: HashMap<UserId, MemberKey>,
        }

        #[derive(Debug, Clone)]
        pub enum ListTarget {
            Room,
            Channel(ChannelId),
        }

        /// a handle for interacting with a member list
        pub struct ListHandle {
            //
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
                                            let old_pos =
                                                self.members.range(..old_key).count() as u64;
                                            ops.push(MemberListOp::Delete {
                                                position: old_pos,
                                                count: 1,
                                            });

                                            self.groups
                                                .get_mut(&old_key.group)
                                                .map(|k| k.count -= 1);
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

            // pub fn subscribe(&self) -> broadcast::Receiver<Arc<MemberListSync>> {
            //     todo!()
            // }
        }
    }

    pub mod syncer {
        use common::{
            v1::types::{MessageSync, SyncSubscribeMemberList},
            v2::types::UserId,
        };

        pub struct ListQuery {
            // pub target: MemberListTarget,
            pub ranges: Vec<(u64, u64)>,
        }

        // pub enum MemberListTarget {
        //     Room(RoomId),
        //     Channel(ChannelId),
        // }

        // pub enum MemberListSync {
        //     Sync {
        //         room_id: Option<RoomId>,
        //         channel_id: Option<ChannelId>,
        //         ops: Vec<MemberListOp>,
        //         groups: Vec<MemberListGroup>,
        //     },
        //     // /// initial ranges for a list
        //     // Initial {},
        // }

        /// manages multiple member lists
        ///
        /// tries to deduplicate data, ie. avoids sending user, room member, and
        /// thread member objects the client already has
        ///
        /// also handles range filtering
        pub struct ListSyncer {
            // TODO: ...
        }

        impl ListSyncer {
            /// set the user id for this syncer
            pub fn set_user_id(&mut self, user_id: Option<UserId>) {
                todo!()
            }

            /// set the subscribed lists for this syncer
            pub fn set_lists(&mut self, _queries: &[SyncSubscribeMemberList]) {
                todo!()
            }

            // /// get a ~~stream~~ mpsc receiver for MessageSync events
            // pub fn subscribe(&self) -> mpsc::Receiver<Arc<MemberListSync>> {
            //     todo!()
            // }

            /// poll for a new sync message
            pub async fn poll(&mut self) -> Result<MessageSync> {
                todo!()
            }
        }
    }
}
