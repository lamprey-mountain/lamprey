use std::{collections::BTreeMap, sync::Arc};

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoleId, RoomId, UserId};
use dashmap::DashMap;
use tokio::{
    sync::{broadcast, mpsc::Receiver, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::{
    services::member_lists::util::{MemberKey, MemberListGroupData, MemberListKey, MemberListKey1},
    Result, ServerStateInner,
};

/// member list actor
pub struct MemberList {
    pub(super) s: Arc<ServerStateInner>,
    pub(super) groups: Vec<MemberListGroupData>,

    pub(super) key: MemberListKey,

    // NOTE: do i need a reverse index? keeping this up to date seems like it could be a pain
    pub(super) user_index: DashMap<UserId, (usize, usize)>,

    // still unsure about this
    pub(super) ordered: BTreeMap<MemberKey, UserId>,
}

/// a handle to a member list actor
pub struct MemberListHandle {
    pub(super) commands: tokio::sync::mpsc::Sender<MemberListCommand>,
    pub(super) events: broadcast::Receiver<MemberListEvent>,
    pub(super) join_handle: JoinHandle<Result<()>>,
}

impl MemberListHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<MemberListEvent> {
        self.events.resubscribe()
    }
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
    pub(super) async fn spawn(self, commands_recv: Receiver<MemberListCommand>) -> Result<()> {
        todo!();
        Ok(())
    }

    pub(super) fn process_event(&mut self, event: &MessageSync) -> Vec<MemberListOp> {
        todo!()
    }

    // TODO: directly port the existing impl for now, write a more efficient one later
    // fn rebuild_groups(&mut self) -> Vec<MemberListOp>
    // pub fn get_initial_ranges(&self, ranges: &[(u64, u64)]) -> Vec<MemberListOp> {
    // pub fn groups(&self) -> Vec<MemberListGroup> {
    // fn remove_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
    // fn find_user(&self, user_id: UserId) -> Option<(usize, usize)> {
    // fn find_group(&self, group_id: MemberListGroupId) -> Option<usize> {
    // fn get_member_group_id(&self, user_id: UserId, is_online: bool) -> MemberListGroupId {
    // fn recalculate_user(&mut self, user_id: UserId) -> Vec<MemberListOp> {
    // fn insert_group(&mut self, group_id: MemberListGroupId) -> usize {
    // fn remove_group(&mut self, group_id: MemberListGroupId) -> Vec<MemberListOp> {

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
