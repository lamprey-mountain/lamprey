// NOTE: this is a rewrite of the original members service
// this implementation will make use of ServiceCache, the new data caching service
// the intention is to eventually migrate list management here

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

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoleId, RoomId, UserId};
use dashmap::DashMap;
use tokio::{
    sync::{
        broadcast,
        mpsc::{self, Receiver},
        oneshot,
    },
    task::JoinHandle,
};
use tokio_stream::StreamMap;
use uuid::Uuid;

use crate::{
    services::member_lists::{
        actor::{MemberList, MemberListHandle},
        syncer::MemberListSyncer,
        util::{MemberKey, MemberListGroupData, MemberListKey, MemberListKey1},
    },
    Result, ServerStateInner,
};

mod actor;
mod syncer;
mod util;
mod visibility;

use visibility::MemberListVisibility;

pub struct ServiceMemberLists {
    s: Arc<ServerStateInner>,
    lists: DashMap<MemberListKey, Arc<MemberListHandle>>,
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

    // TODO: remove? unsure what i'd use this for
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
            conn_id,
            outbox: VecDeque::new(),
            streams: StreamMap::new(),
        }
    }

    async fn ensure(&self, key: MemberListKey) -> Result<Arc<MemberListHandle>> {
        let (commands_send, commands_recv) = mpsc::channel(100);
        let (events_send, events_recv) = broadcast::channel(100);
        let list = MemberList {
            s: self.s.clone(),
            groups: vec![],
            key: key.clone(),
            user_index: todo!(),
            ordered: todo!(),
        };
        let join_handle = tokio::spawn(list.spawn(commands_recv));
        let handle = Arc::new(MemberListHandle {
            commands: commands_send,
            events: events_recv,
            join_handle,
        });
        self.lists.insert(key, Arc::clone(&handle));
        Ok(handle)
    }
}
