//! Experimental rewrite for member management
//!
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
//! - MemberListKey1: (wip name)

#![allow(unused)] // TEMP: suppress warnings here for now

use std::{collections::BTreeMap, sync::Arc};

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoleId, RoomId, UserId};
use dashmap::DashMap;
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::{
    services::member_lists::util::{MemberKey, MemberListGroupData, MemberListKey, MemberListKey1},
    Result, ServerStateInner,
};

mod util;
mod visibility;
// mod syncer; // TODO: move MemberListSyncer here
// mod actor; // TODO: move MemberList here

use visibility::MemberListVisibility;

pub struct ServiceMemberLists {
    s: Arc<ServerStateInner>,
    lists: DashMap<MemberListKey, Arc<MemberListHandle>>,
}

/// member list actor
struct MemberList {
    s: Arc<ServerStateInner>,
    groups: Vec<MemberListGroupData>,

    key: MemberListKey,

    // NOTE: do i need a reverse index? keeping this up to date seems like it could be a pain
    user_index: DashMap<UserId, (usize, usize)>,

    // still unsure about this
    ordered: BTreeMap<MemberKey, UserId>,
}

/// a handle to a member list actor
struct MemberListHandle {
    commands: tokio::sync::mpsc::Sender<MemberListCommand>,
    // events: tokio::sync::mpsc::Receiver<ActorEvent>,
    join_handle: JoinHandle<Result<()>>,
}

enum MemberListCommand {
    GetInitialRanges {
        ranges: Vec<(u64, u64)>,
        callback: oneshot::Sender<MessageSync>,
    },
}

pub struct MemberListSyncer {
    s: Arc<ServerStateInner>,
    // query_tx: tokio::sync::watch::Sender<Option<MemberListQuery>>,
    // query_rx: tokio::sync::watch::Receiver<Option<MemberListQuery>>,
    // // current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,
    // conn_id: Uuid,
    // outbox: VecDeque<MessageSync>,
}

impl ServiceMemberLists {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            s: state,
            lists: todo!(),
        }
    }

    pub async fn lookup_member_key(&self, key1: MemberListKey1) -> Result<MemberListKey> {
        // get channel, check if is dm, is thread
        todo!()
    }

    // remove?
    pub fn handle_event(&self, msg: &MessageSync) {
        todo!()
        // match msg {
        //     MessageSync::RoomMemberUpdate { member } => {
        //         if let Some(map) = self.room_members.get(&member.room_id) {
        //             if let Some(_existing) = map.get_mut(&member.user_id) {
        //                 // this doesn't work, i can't mutate _existing
        //                 // existing.override_name = member.override_name.map(|s| Arc::<str>::from(s));
        //                 // existing.deaf = member.deaf;
        //                 // existing.mute = member.mute;
        //                 // existing.roles = member.roles.clone();
        //                 // existing.timeout_until = member.timeout_until;
        //                 // TODO: recalculate lists
        //             }
        //         }
        //     }
        //     MessageSync::UserUpdate { user } => {
        //         let name: Arc<str> = Arc::from(user.name.as_str());
        //         for map in &self.room_members {
        //             if let Some(mut _existing) = map.get_mut(&user.id) {
        //                 // existing.user_name = Arc::clone(&name);
        //                 // TODO: recalculate lists
        //             }
        //         }
        //     }
        //     _ => {}
        // }
    }

    /// create a new MemberListSyncer for a connection
    pub fn create_syncer(&self, conn_id: Uuid) -> MemberListSyncer {
        // let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        MemberListSyncer {
            s: self.s.clone(),
            // query_tx,
            // query_rx,
            // current_rx: None,
            // conn_id,
            // outbox: VecDeque::new(),
        }
    }

    async fn ensure(&self, key1: MemberListKey1) -> Result<Arc<MemberListHandle>> {
        let key = self.lookup_member_key(key1).await?;
        let list = MemberList {
            s: self.s.clone(),
            groups: vec![],
            key: key.clone(),
            user_index: todo!(),
            ordered: todo!(),
        };
        let (commands_send, commands_recv) = tokio::sync::mpsc::channel(100);
        let join_handle = tokio::spawn(list.spawn(commands_recv));
        let handle = Arc::new(MemberListHandle {
            commands: commands_send,
            join_handle,
        });
        self.lists.insert(key, Arc::clone(&handle));
        Ok(handle)
    }
}

impl MemberList {
    /// whether this list should be restricted to thread members instead of using room member permission logic
    pub fn use_thread_members(&self) -> bool {
        match self.key {
            MemberListKey::Room(..) => false,
            MemberListKey::RoomChannel(..) => false,
            MemberListKey::RoomThread(..) => true,
            MemberListKey::Dm(..) => true,
        }
    }
}

impl MemberList {
    async fn spawn(self, commands_recv: Receiver<MemberListCommand>) -> Result<()> {
        todo!();
        Ok(())
    }

    fn process_event(&mut self, event: &MessageSync) -> Vec<MemberListOp> {
        todo!()
    }

    /// recalculate groups from scratch
    fn rebuild_groups(&mut self) -> Vec<MemberListOp> {
        todo!()
    }

    // pub fn get_initial_ranges(&self, ranges: &[(u64, u64)]) -> Vec<MemberListOp> {
    // pub fn groups(&self) -> Vec<MemberListGroup> {
    // fn remove_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
    // fn find_user(&self, user_id: UserId) -> Option<(usize, usize)> {
    // fn find_group(&self, group_id: MemberListGroupId) -> Option<usize> {
    // fn get_member_group_id(&self, user_id: UserId, is_online: bool) -> MemberListGroupId {
    // fn recalculate_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
    // fn insert_group(&mut self, group_id: MemberListGroupId) -> usize {
    // fn remove_group(&mut self, group_id: MemberListGroupId) -> Vec<MemberListOp> {
}

// NOTE: user_id is remove, auth checks should be done in syncer
impl MemberListSyncer {
    pub fn subscribe(&self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
        todo!()
    }

    pub fn unsubscribe(&self, key1: MemberListKey1) -> Result<()> {
        todo!()
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        todo!()
    }
}
