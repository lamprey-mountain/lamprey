//! Experimental rewrite for member management; unlikely to be production ready anytime soon

#![allow(unused)] // TEMP: suppress warnings here for now

use std::{collections::BTreeMap, sync::Arc};

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoleId, RoomId, UserId};
use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    services::{
        members::MemberListQuery,
        members2::summary::{RoleSummary, RoomMemberSummary, ThreadMemberSummary},
    },
    Result, ServerStateInner,
};

mod summary;
mod visibility;

use visibility::MemberListVisibility;

pub struct ServiceMembers {
    s: Arc<ServerStateInner>,
    room_members: DashMap<RoomId, DashMap<UserId, Arc<RoomMemberSummary>>>,
    thread_members: DashMap<ChannelId, DashMap<UserId, Arc<ThreadMemberSummary>>>,
    // NOTE: maybe i want to include id in role and store these as a vec?
    roles: DashMap<RoomId, DashMap<RoleId, Arc<RoleSummary>>>,
    lists: DashMap<MemberListKey, MemberList>,
}

// maybe i could store rooms for everything
// struct Room {
//     room_members: DashMap<UserId, Arc<RoomMemberSummary>>,
//     // thread_members: DashMap<ChannelId, DashMap<UserId, Arc<ThreadMemberSummary>>>,
//     // NOTE: maybe i want to include id in role and store these as a vec?
//     roles: Vec<Role>,
//     lists: DashMap<MemberListKey, MemberList>,
// }

/// for deduplicating member lists
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MemberListKey {
    /// the entire member list of a room
    Room(RoomId),

    /// a channel in a room
    RoomChannel(RoomId, MemberListVisibility),

    /// a thread in a room's channel
    RoomThread(RoomId, MemberListVisibility, ChannelId),

    /// a dm channel
    ///
    /// (maybe remove later?)
    Dm(ChannelId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberGroupInfo {
    Online,
    Offline,
    Hoisted(RoleId),
}

#[derive(Debug)]
struct MemberListGroupData {
    info: MemberGroupInfo,
    users: Vec<UserId>,
}

struct MemberList {
    s: Arc<ServerStateInner>,
    groups: Vec<MemberListGroupData>,

    key: MemberListKey,

    // NOTE: do i need a reverse index? keeping this up to date seems like it could be a pain
    user_index: DashMap<UserId, (usize, usize)>,

    // still unsure about this
    ordered: BTreeMap<MemberKey, UserId>,
}

#[derive(Debug, PartialEq, Eq)]
struct MemberKey {
    /// role position, -1 used for online, -2 used for offline
    role_pos: i64,

    /// either the override_name or user name
    name: Arc<str>,
}

impl PartialOrd for MemberKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.role_pos.cmp(&other.role_pos) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        self.name.cmp(&other.name)
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

pub struct MemberListSyncer {
    s: Arc<ServerStateInner>,
    // query_tx: tokio::sync::watch::Sender<Option<MemberListQuery>>,
    // query_rx: tokio::sync::watch::Receiver<Option<MemberListQuery>>,
    // // current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,
    // conn_id: Uuid,
    // outbox: VecDeque<MessageSync>,
    // NOTE: remove user_id, push auth checks up to syncer
}

impl ServiceMembers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            s: state,
            room_members: todo!(),
            thread_members: todo!(),
            roles: todo!(),
            lists: todo!(),
        }
    }

    pub async fn lookup_member_key(
        &self,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
    ) -> Result<MemberListKey> {
        // fail if room_id and channel_id are both None
        // get channel, check if is dm, is thread
        todo!()
    }

    pub fn handle_event(&self, msg: &MessageSync) {
        match msg {
            MessageSync::RoomMemberUpdate { member } => {
                if let Some(map) = self.room_members.get(&member.room_id) {
                    if let Some(_existing) = map.get_mut(&member.user_id) {
                        // this doesn't work, i can't mutate _existing
                        // existing.override_name = member.override_name.map(|s| Arc::<str>::from(s));
                        // existing.deaf = member.deaf;
                        // existing.mute = member.mute;
                        // existing.roles = member.roles.clone();
                        // existing.timeout_until = member.timeout_until;
                        // TODO: recalculate lists
                    }
                }
            }
            MessageSync::UserUpdate { user } => {
                let name: Arc<str> = Arc::from(user.name.as_str());
                for map in &self.room_members {
                    if let Some(mut _existing) = map.get_mut(&user.id) {
                        // existing.user_name = Arc::clone(&name);
                        // TODO: recalculate lists
                    }
                }
            }
            _ => {}
        }
    }

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

    async fn spawn(&self, room_id: Option<RoomId>, channel_id: Option<ChannelId>) -> Result<()> {
        let list = MemberList {
            s: self.s.clone(),
            groups: vec![],
            key: todo!(),
            user_index: todo!(),
            ordered: todo!(),
        };
        Ok(())
    }

    // pub async fn subscribe(
    //     &self,
    //     query: Query,
    // ) -> Result<broadcast::Receiver<DocumentEvent>> {
    //     let ctx = self.load(context_id, None).await?;
    //     let ctx = ctx.read().await;
    //     Ok(ctx.update_tx.subscribe())
    // }

    // thread_member_list
    // thread_member_get
    // thread_member_add
    // thread_member_delete
    // room_member_list
    // room_member_get
    // room_member_add
    // room_member_update
    // room_member_delete
    // room_member_search
    // room_member_search_advanced
    // room_member_prune
}

impl MemberList {
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

impl MemberListSyncer {
    pub fn set_user_id(&self) -> Result<()> {
        todo!()
    }

    pub fn set_query(&self, query: Option<MemberListQuery>) -> Result<()> {
        todo!()
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        todo!()
    }
}
