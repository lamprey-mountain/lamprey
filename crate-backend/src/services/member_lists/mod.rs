//! Service for managing member lists
//!
//! ## Member list logic
//!
//! In threads, the active member set is all members who are have an associated
//! thread_member object. In other channels, a member is active if they can view
//! the channel.
//!
//! A group is formed for each hoisted role, online members, and offline members.
//! Role groups are returned first (ordered by position), followed by online
//! members, then finally by offline members. A member is part of group formed by
//! their highest hoisted role. Role groups only contain online members, offline
//! members are always part of the offline group regardless of roles. If a group
//! has no members, it is not returned.
//!
//! After the member sets are filtered and grouped, they are ordered by their
//! display name. The display name uses the room override_name, falling back to
//! user name.
//!
//! ## Architecture
//!
//! - ServiceMemberLists: main entrypoint into member list management
//! - MemberList: a single spawned actor
//! - MemberListHandle: a way to control one MemberList actor
//! - MemberListSyncer: created per ws sync connection
//! - MemberListKey: an identifier for a single list. lists are deduplicated by visibility.
//! - MemberListKey1: (wip name) a member list identifier from the api

use std::collections::HashSet;
use std::sync::Arc;

use common::v1::types::{MessageSync, RoomId, User};
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::{
    services::member_lists::{
        actor::{MemberList, MemberListHandle},
        util::{MemberListKey, MemberListKey1},
        visibility::MemberListVisibility,
    },
    Result, ServerStateInner,
};

mod actor;
pub mod syncer;
pub mod util;
pub mod visibility;

/// Service for managing member lists
pub struct ServiceMemberLists {
    s: Arc<ServerStateInner>,
    lists: DashMap<MemberListKey, Arc<MemberListHandle>>,
    room_to_lists: DashMap<RoomId, HashSet<MemberListKey>>,
}

impl ServiceMemberLists {
    /// Create a new member lists service
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            s: state,
            lists: DashMap::new(),
            room_to_lists: DashMap::new(),
        }
    }

    /// Lookup a member list key from an API key
    pub async fn lookup_member_key(&self, key1: MemberListKey1) -> Result<MemberListKey> {
        let srv = self.s.services();
        match key1 {
            MemberListKey1::Room(room_id) => Ok(MemberListKey::Room(room_id)),
            MemberListKey1::RoomChannel(room_id, channel_id) => {
                let chan = srv.channels.get(channel_id, None).await?;
                if chan.is_thread() && chan.ty.member_list_uses_thread_members() {
                    return Ok(MemberListKey::RoomThread(
                        room_id,
                        MemberListVisibility::default(),
                        channel_id,
                    ));
                }
                let overwrites = srv.channels.fetch_overwrite_ancestors(channel_id).await?;
                let visibility = MemberListVisibility::from_overwrites(room_id, overwrites);
                Ok(MemberListKey::RoomChannel(room_id, visibility))
            }
            MemberListKey1::DmChannel(channel_id) => Ok(MemberListKey::Dm(channel_id)),
        }
    }

    /// Handle a sync event and notify affected member lists
    pub async fn handle_event(&self, msg: &MessageSync) {
        let affected_lists = match msg {
            MessageSync::RoomMemberCreate { member, .. }
            | MessageSync::RoomMemberUpdate { member, .. } => self
                .room_to_lists
                .get(&member.room_id)
                .map(|s| s.value().clone())
                .unwrap_or_default(),
            MessageSync::RoomMemberDelete { room_id, .. } => self
                .room_to_lists
                .get(room_id)
                .map(|s| s.value().clone())
                .unwrap_or_default(),
            MessageSync::ChannelUpdate { channel } => {
                // if visibility changed, we might need to notify multiple lists or move channels between lists
                // for now, let's just trigger a re-check for the room
                if let Some(room_id) = channel.room_id {
                    self.room_to_lists
                        .get(&room_id)
                        .map(|s| s.value().clone())
                        .unwrap_or_default()
                } else {
                    HashSet::new()
                }
            }
            MessageSync::PresenceUpdate { user_id, .. }
            | MessageSync::UserUpdate {
                user: User { id: user_id, .. },
            } => {
                let user_id = *user_id;
                let mut affected = HashSet::new();
                for entry in self.room_to_lists.iter() {
                    let room_id = *entry.key();
                    if let Ok(room) = self.s.services().cache.load_room(room_id).await {
                        if room.members.contains_key(&user_id) {
                            affected.extend(entry.value().clone());
                        }
                    }
                }
                affected
            }
            _ => return,
        };

        for key in affected_lists {
            if let Some(handle) = self.lists.get(&key) {
                let _ = handle.sync_tx.send(msg.clone()).await;
            }
        }
    }

    /// Ensure a member list exists and return its handle
    pub async fn ensure(&self, key: MemberListKey) -> Result<Arc<MemberListHandle>> {
        if let Some(handle) = self.lists.get(&key) {
            return Ok(Arc::clone(handle.value()));
        }

        let (commands_tx, commands_recv) = mpsc::channel(100);
        let (sync_tx, sync_recv) = mpsc::channel(100);
        let (events_tx, _) = tokio::sync::broadcast::channel(100);

        let list = MemberList {
            s: self.s.clone(),
            key: key.clone(),
            members: Default::default(),
            user_to_key: Default::default(),
            groups: vec![],
            events_tx: events_tx.clone(),
        };

        if let Some(room_id) = match &key {
            MemberListKey::Room(id) => Some(*id),
            MemberListKey::RoomChannel(id, _) => Some(*id),
            MemberListKey::RoomThread(id, _, _) => Some(*id),
            MemberListKey::Dm(_) => None,
        } {
            self.room_to_lists
                .entry(room_id)
                .or_default()
                .insert(key.clone());
        }

        let handle = Arc::new(MemberListHandle {
            commands_tx,
            sync_tx,
            events_tx,
            join_handle: tokio::spawn(list.spawn(commands_recv, sync_recv)),
        });

        self.lists.insert(key, Arc::clone(&handle));
        Ok(handle)
    }

    /// Create a new syncer for a connection
    pub fn create_syncer(&self, conn_id: uuid::Uuid) -> syncer::MemberListSyncer {
        syncer::MemberListSyncer::new(self.s.clone(), conn_id)
    }

    /// Start background tasks for the service
    pub fn start_background_tasks(&self) {
        let s = self.s.clone();
        tokio::spawn(async move {
            let mut sushi = s.subscribe_sushi().await.unwrap();
            while let Some(msg) = sushi.next().await {
                s.services().member_lists.handle_event(&msg.message).await;
            }
        });
    }
}
